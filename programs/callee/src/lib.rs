use anchor_lang::prelude::*;

pub mod interface;
pub mod processor;
pub mod state;

// Need this to be able to get ExternalIAccountMeta into IDLs
// Please remove once ExternalIAccountMeta is a normal type in Anchor
use processor::create_linked_list::*;
use processor::create_ownership_list::*;
use processor::return_data::*;
use processor::transfer_linked_list::*;
use processor::transfer_ownership_list::*;

declare_id!("8hKjTVHaCE4U2zMYVx5eu5P9MTCU2imhvZZU31jDnYNA");

#[program]
pub mod callee {
    use super::*;

    /// Transfers all nodes in a linked list to a different owner
    pub fn transfer_linked_list<'info>(
        ctx: Context<'_, '_, 'info, 'info, TransferLinkedList<'info>>,
        destination: Pubkey,
    ) -> Result<()> {
        processor::transfer_linked_list::transfer_linked_list(ctx, destination)
    }

    pub fn preflight_transfer_linked_list<'info>(
        ctx: Context<'_, '_, 'info, 'info, TransferLinkedList<'info>>,
        destination: Pubkey,
    ) -> Result<()> {
        processor::transfer_linked_list::preflight_transfer_linked_list(ctx, destination)
    }

    pub fn transfer_ownership_list<'info>(
        ctx: Context<'_, '_, 'info, 'info, TransferOwnershipList<'info>>,
        destination: Pubkey,
    ) -> Result<()> {
        processor::transfer_ownership_list::transfer_ownership_list(ctx, destination)
    }

    pub fn preflight_transfer_ownership_list<'info>(
        ctx: Context<'_, '_, 'info, 'info, TransferOwnershipList<'info>>,
        destination: Pubkey,
    ) -> Result<()> {
        processor::transfer_ownership_list::preflight_transfer_ownership_list(ctx, destination)
    }

    /// Boilerplate initialization methods
    /// Test account data introspection
    pub fn create_linked_list<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateLinkedList<'info>>,
        num: u32,
    ) -> Result<()> {
        processor::create_linked_list::create_linked_list(ctx, num)
    }

    /// Test usefulness of account paging
    pub fn create_ownership_list<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateOwnershipList<'info>>,
        num: u32,
    ) -> Result<()> {
        processor::create_ownership_list::create_ownership_list(ctx, num)
    }

    /// Utilities
    /// Return data
    pub fn return_data<'info>(ctx: Context<'_, '_, 'info, 'info, Noop>, amount: u32) -> Result<()> {
        processor::return_data::return_data(ctx, amount)
    }
}
