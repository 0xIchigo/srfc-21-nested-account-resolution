/** @type {import('ts-jest').JestConfigWithTsJest} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  verbose: true,
  testMatch: ["**/tests/nested-account-resolution.ts"],
  testTimeout: 10000,
};
