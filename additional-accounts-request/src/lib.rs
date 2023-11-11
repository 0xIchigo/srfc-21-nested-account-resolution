use anchor_lang::prelude::*;
use anchor_lang::solana_program::log::sol_log_compute_units;
use anchor_lang::solana_program::{
    hash,
    program::{get_return_data, invoke, invoke_signed, set_return_data},
};

use bytemuck::cast_slice;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct IAccountMeta {
    pub pubkey: Pubkey,
    pub signer: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct AdditionalAccountsRequest {
    pub accounts: AdditionalAccounts,
    pub page: u8,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct AdditionalAccounts {
    pub accounts: Vec<IAccountMeta>,
    pub has_more: bool,
}

const MAX_ACCOUNTS: usize = 29;

impl AdditionalAccounts {
    pub fn page_to(&mut self, requested_page: u8) -> Result<()> {
        if requested_page * 29 > self.accounts.len() as u8 {
            msg!("Invalid page");
            return Err(ProgramError::Custom(696969).into());
        }
        // In case we return to a previous page, we need to remove all accounts
        if requested_page > 0 {
            self.accounts
                .drain(0..requested_page as usize * MAX_ACCOUNTS);
        }
        if self.accounts.len() > MAX_ACCOUNTS {
            self.accounts.truncate(self.accounts.len() - MAX_ACCOUNTS);
        }
        Ok(())
    }

    // pub fn set_return_data(&self) {
    //     let mut data = [0u8; MAX_RETURN_DATA];

    //     let writer = &mut data;
    //     let len_bytes = (self.accounts.len() as u32).to_le_bytes();
    //     for (i, byte) in len_bytes.iter().enumerate() {
    //         writer[i] = *byte;
    //     }

    //     for i in 0..self.accounts.len() {
    //         let account = self.accounts.get(i).unwrap();
    //         let account_key_bytes = bytemuck::bytes_of(&account.pubkey);

    //         let start_idx = 4 + 34 * i;
    //         for (j, byte) in account_key_bytes.iter().enumerate() {
    //             writer[start_idx + j] = *byte;
    //         }

    //         writer[start_idx + 32] = if account.signer { 1 } else { 0 };
    //         writer[start_idx + 33] = if account.writable { 1 } else { 0 };
    //     }

    //     set_return_data(&data);
    // }
}

pub fn identify_additional_accounts<'info, C1: ToAccountInfos<'info> + ToAccountMetas>(
    ix_name: String,
    ctx: &CpiContext<'_, '_, '_, 'info, C1>,
    args: &[u8],
    log_info: bool,
) -> Result<AdditionalAccounts> {
    if log_info {
        msg!("Preflight {}", &ix_name);
    }

    let mut additional_accounts: Vec<IAccountMeta> = vec![];

    let mut has_more = true;
    let mut page = 0;
    while has_more {
        if log_info {
            msg!("Page: {} {} {}", page, has_more, additional_accounts.len());
        }
        call_preflight_interface_function(ix_name.clone(), &ctx, &args, page)?;

        let program_key = ctx.program.key();
        let (key, program_data) = get_return_data().unwrap();
        assert_eq!(key, program_key);

        let program_data = program_data.as_slice();
        if log_info {
            msg!("Return data length: {}", program_data.len());
        }
        let accs = AdditionalAccounts::try_from_slice(&program_data)?;
        // if log_info {
        //     msg!("Additional accounts: {:?}", &accs);
        // }

        additional_accounts.extend(accs.accounts);

        let mut should_exit = false;
        additional_accounts.iter().rev().for_each(|acc| {
            let mut found = false;
            ctx.remaining_accounts.iter().rev().for_each(|account| {
                if account.key == &acc.pubkey {
                    found = true;
                }
            });
            if !found {
                should_exit = true;
            }
        });
        if should_exit {
            msg!("Missing account(s)");
            break;
        }

        has_more = accs.has_more;
        page += 1;
    }

    Ok(AdditionalAccounts {
        accounts: additional_accounts,
        has_more,
    })
}

/// This calls the preflight function on the target program (defined on the ctx)
pub fn call_preflight_interface_function<'info, T: ToAccountInfos<'info> + ToAccountMetas>(
    function_name: String,
    ctx: &CpiContext<'_, '_, '_, 'info, T>,
    args: &[u8],
    page: u8,
) -> Result<()> {
    // setup
    sol_log_compute_units();
    let mut ix_data: Vec<u8> =
        hash::hash(format!("global:preflight_{}", &function_name).as_bytes()).to_bytes()[..8]
            .to_vec();

    ix_data.extend_from_slice(args);
    ix_data.extend_from_slice(&[page]);

    let mut ix_account_metas = ctx.accounts.to_account_metas(Some(false));
    ix_account_metas.extend(ctx.remaining_accounts.to_account_metas(None));

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.program.key(),
        accounts: ix_account_metas,
        data: ix_data,
    };
    sol_log_compute_units();
    msg!("Preflighted...");

    // execute
    let mut ix_ais = ctx.accounts.to_account_infos();
    ix_ais.extend(ctx.remaining_accounts.to_account_infos());
    invoke(&ix, &ix_ais)?;
    Ok(())
}

/// This calls the main function on the target program, and passes along the requested
/// account_metas from the preflight function
pub fn call_interface_function<'info, T: ToAccountInfos<'info> + ToAccountMetas>(
    function_name: String,
    ctx: CpiContext<'_, '_, '_, 'info, T>,
    args: &[u8],
    additional_accounts: Vec<IAccountMeta>,
    log_info: bool,
) -> Result<()> {
    if log_info {
        msg!("Creating interface context...");
        sol_log_compute_units();
    }
    // setup
    let remaining_accounts = ctx.remaining_accounts.to_vec();

    let mut ix_data: Vec<u8> =
        hash::hash(format!("global:{}", &function_name).as_bytes()).to_bytes()[..8].to_vec();
    ix_data.extend_from_slice(&args);

    if log_info {
        msg!("Account Metas creation...");
        sol_log_compute_units();
    }
    let mut ix_account_metas = ctx.accounts.to_account_metas(None);
    ix_account_metas.append(
        additional_accounts
            .iter()
            .map(|acc| {
                if acc.writable {
                    AccountMeta::new(acc.pubkey, acc.signer)
                } else {
                    AccountMeta::new_readonly(acc.pubkey, acc.signer)
                }
            })
            .collect::<Vec<AccountMeta>>()
            .as_mut(),
    );
    if log_info {
        sol_log_compute_units();
        msg!("Account Metas created...");
    }

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.program.key(),
        accounts: ix_account_metas,
        data: ix_data,
    };

    let mut ix_ais: Vec<AccountInfo> = ctx.accounts.to_account_infos();
    if log_info {
        msg!("IX accounts: {:?}", &ix_ais.len());
        msg!("Account Info creation...");
        sol_log_compute_units();
    }
    // Oddly enough, we only need to specify the account metas
    // we can just throw the account infos in there and account metas
    // will specify ordering & filtering (?)
    ix_ais.extend_from_slice(&remaining_accounts);
    if log_info {
        sol_log_compute_units();
        msg!("Account Infos created...");
    }

    if log_info {
        msg!("IX accounts: {:?}", &ix_ais.len());
        // ix_ais.iter().into_iter().for_each(|ai| {
        //     msg!(
        //         "Account: {:?}, {:?}, {:?}, {:?}",
        //         ai.key,
        //         ai.owner,
        //         ai.is_signer,
        //         ai.is_writable
        //     )
        // });
        // msg!("Signer seeds: {:?}", &ctx.signer_seeds);
    }

    if log_info {
        msg!("Finished creating context...");
        sol_log_compute_units();
    }

    // execute
    invoke_signed(&ix, &ix_ais, &ctx.signer_seeds)?;
    Ok(())
}

/// Calls an instruction on a program that complies with the additional accounts interface
///
/// Expects ctx.remaining accounts to have all possible accounts in order to resolve
/// the accounts requested from the preflight function
pub fn call<'info, C1: ToAccountInfos<'info> + ToAccountMetas>(
    ix_name: String,
    ctx: CpiContext<'_, '_, '_, 'info, C1>,
    args: Vec<u8>,
    log_info: bool,
) -> Result<()> {
    // preflight
    if log_info {
        msg!("Identifying additional accounts...")
    }
    let additional_accounts = identify_additional_accounts(ix_name.clone(), &ctx, &args, log_info)?;

    // execute
    if log_info {
        msg!("Execute {}", &ix_name);
    }
    call_interface_function(
        ix_name.clone(),
        ctx,
        &args,
        additional_accounts.accounts,
        log_info,
    )?;
    Ok(())
}

pub fn call_preflight_interface_function_faster<'info>(
    function_name: String,
    program_key: &Pubkey,
    account_infos: &[AccountInfo<'info>],
    account_metas: Vec<AccountMeta>,
    args: &[u8],
) -> Result<()> {
    // setup
    sol_log_compute_units();
    let mut ix_data: Vec<u8> =
        hash::hash(format!("global:preflight_{}", &function_name).as_bytes()).to_bytes()[..8]
            .to_vec();

    ix_data.extend_from_slice(args);

    // let ix_account_metas = account_metas;
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: program_key.clone(),
        accounts: account_metas,
        data: ix_data,
    };
    sol_log_compute_units();
    msg!("Preflighted...");

    // execute
    invoke(&ix, &account_infos)?;
    Ok(())
}

// TODO(ngundotra): write this without any anchor stuff, and see if just moving slices around is faster
pub fn call_faster<'info>(
    ix_name: String,
    program_key: Pubkey,
    account_infos: Vec<AccountInfo<'info>>,
    account_metas: Vec<AccountMeta>,
    remaining_accounts: &[AccountInfo<'info>],
    signer_seeds: &[&[&[u8]]],
    args: Vec<u8>,
    verbose: bool,
) -> Result<()> {
    // preflight
    call_preflight_interface_function_faster(
        ix_name.clone(),
        &program_key,
        &account_infos,
        account_metas.clone(),
        &args,
    )?;
    msg!("Begin");
    sol_log_compute_units();

    let (key, program_data) = get_return_data().unwrap();
    assert_eq!(key, program_key);

    let program_data = program_data.as_slice();
    let num_accounts = u32::try_from_slice(&program_data[..4])?;

    let mut ix_ais: Vec<AccountInfo> =
        Vec::with_capacity(account_infos.len() + num_accounts as usize);
    ix_ais.extend_from_slice(&account_infos);
    let mut ix_account_metas: Vec<AccountMeta> =
        Vec::with_capacity(account_metas.len() + num_accounts as usize);
    ix_account_metas.extend_from_slice(&account_metas);

    // Maps from the requested_account to its ordering in remaining accounts
    // let remaining_accounts = ctx.remaining_accounts.as_slice();
    let mut num_found: u32 = 0;
    let mut account_popped = vec![false; remaining_accounts.len()];
    for account_idx in 0..num_accounts {
        let start_idx = 4 + account_idx as usize * 34;
        let end_idx = 4 + (account_idx as usize + 1) * 34;

        // let requested_account_meta =
        // IAccountMeta::try_from_slice(&program_data[start_idx as usize..end_idx as usize])?;
        let pubkey = cast_slice::<u8, Pubkey>(&program_data[start_idx..end_idx - 2])[0];
        let is_signer: bool = program_data[end_idx - 2] == 1u8;
        let is_writable: bool = program_data[end_idx - 1] == 1u8;

        ix_account_metas.push(AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        });

        // Yes this is O(M*N)
        // M = len(requested accounts)
        // N = len(remaining accounts)
        // But in practice, this is faster than using hashmap bc CU fees
        // NOTE: this does not work if requested_accounts has duplicates
        let mut floating_idx = 0;
        for floating_acc in remaining_accounts {
            if account_popped[floating_idx] {
                floating_idx += 1;
                continue;
            }
            if floating_acc.key == &pubkey {
                ix_ais.push(floating_acc.clone());
                num_found += 1;

                // Only add account once, then break
                account_popped[floating_idx] = true;
                break;
            }
            floating_idx += 1;
        }
    }
    sol_log_compute_units();

    if num_found != num_accounts {
        msg!(
            "Could not find account infos for requested accounts. Found {}, expected {}",
            num_found,
            num_accounts
        );
        return Err(ProgramError::InvalidAccountData.into());
    }

    let mut ix_data: Vec<u8> =
        hash::hash(format!("global:{}", &ix_name).as_bytes()).to_bytes()[..8].to_vec();
    ix_data.extend_from_slice(&args);

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: program_key.clone(),
        accounts: ix_account_metas,
        data: ix_data,
    };

    msg!("Fin...");
    sol_log_compute_units();

    invoke_signed(&ix, &ix_ais, signer_seeds)?;

    Ok(())
}

pub fn forward_return_data(expected_program_key: &Pubkey) {
    let (key, return_data) = get_return_data().unwrap();
    assert_eq!(key, *expected_program_key);
    set_return_data(&return_data);
}

pub trait InterfaceInstruction {
    fn instruction_name() -> String;
}
