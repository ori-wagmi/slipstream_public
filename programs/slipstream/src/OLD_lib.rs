use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
pub const MAXIMUM_SIZE: usize = (32 * 2) + 1 + 8 + 8 + 8;

#[program]
pub mod slipstream {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, multisig: Pubkey, team_wallet: Pubkey, veststart: u64, vestlength: u64) -> Result<()> {
        // need to create PDA based off input seed
        let acc = &mut ctx.accounts.data_account;
        acc.wallets = [multisig,team_wallet];
        acc.is_frozen = false;
        acc.vesting_start_time = veststart;
        acc.vesting_length = vestlength;
        Ok(())
    }

    pub fn get_pending_claim(ctx: Context<Arguments>) -> Result<u64> {
        let acc = &ctx.accounts.data_account;
        let current_timestamp = Clock::get()?.unix_timestamp as u64;
        let lamports = **(acc.to_account_info().try_borrow_lamports().unwrap());
        if current_timestamp < acc.vesting_start_time {
            return Err(ErrorCode::VestingNotStarted.into());
        }

        let pending_sol: u64;
        if current_timestamp < (acc.vesting_start_time + acc.vesting_length) {
            // (currentTime - vestingStartTime) / vestingLength * depositedSOL
            pending_sol = (current_timestamp - acc.vesting_start_time) * lamports / acc.vesting_length; // this does integer arithmtic, pls fix.
        }
        else {
            pending_sol = lamports;
        }

        Ok(pending_sol)
    }


    pub fn claim_sol(ctx: Context<Arguments>) -> Result<()> {
        let acc = ctx.accounts.data_account.clone();
        if acc.is_frozen {
            return Err(ErrorCode::FundsFrozen.into());
        }

        let to_acc = ctx.accounts.input_account.to_account_info();
        if to_acc.key != &acc.wallets[1] {
            return Err(ErrorCode::NotTeamAccount.into());
        }

        let lamports_to_claim = get_pending_claim(ctx);
        transfer_lamports(&acc.to_account_info(), &to_acc, lamports_to_claim.unwrap()).unwrap();
        Ok(())
    }

    pub fn freeze_funds(ctx: Context<Arguments>, is_frozen: bool) -> Result<()> {
        let acc = &mut ctx.accounts.data_account;
        let multisig_acc = ctx.accounts.input_account.to_account_info();
        if multisig_acc.key != &acc.wallets[0] {
            return Err(ErrorCode::NotMultisig.into());
        }

        acc.is_frozen = is_frozen;
        Ok(())
    }

    pub fn initiate_refund(ctx: Context<Arguments>) -> Result<()> {
        let acc = &mut ctx.accounts.data_account;
        if !acc.is_frozen {
            return Err(ErrorCode::FundsNotFrozen.into());
        }

        let multisig_acc = ctx.accounts.input_account.to_account_info();
        if multisig_acc.key != &acc.wallets[0] {
            return Err(ErrorCode::NotMultisig.into());
        }

        let lamports = **(acc.to_account_info().try_borrow_lamports().unwrap());
        transfer_lamports(&acc.to_account_info(), &multisig_acc, lamports).unwrap();
        Ok(())
    }
}


#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = owner, space = 8+MAXIMUM_SIZE)]
    pub data_account: Account<'info, Data>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// I don't know how function arguments work so i'm stuffing everything in here.
#[derive(Accounts)]
pub struct Arguments<'info> {
    #[account(mut)]
    pub data_account: Account<'info, Data>, // the PDA
    pub input_account: Account<'info, Data>, // caller
}

#[account]
pub struct Data {
    wallets: [Pubkey; 2],       // (32 * 2), MultiSig | TeamWallet
    is_frozen: bool,            // 1
    vesting_start_time: u64,    // 8
    vesting_length: u64,        // 8
}

#[error_code]
pub enum ErrorCode {
    #[msg("insufficient funds for transaction.")]
    InsufficientFundsForTransaction,
    #[msg("only Team can claim.")]
    NotTeamAccount,
    #[msg("caller is not multisig.")]
    NotMultisig,
    #[msg("funds are frozen")]
    FundsFrozen,
    #[msg("funds not frozen")]
    FundsNotFrozen,
    #[msg("vesting not started")]
    VestingNotStarted,
}

/// Copied from solana cookbook
// Transfers lamports from one account (must be program owned)
// to another account. The recipient can by any account
fn transfer_lamports(
    from_account: &AccountInfo,
    to_account: &AccountInfo,
    amount_of_lamports: u64,
) -> Result<()> {
    // Does the from account have enough lamports to transfer?
    if **from_account.try_borrow_lamports()? < amount_of_lamports {
        return Err(ErrorCode::InsufficientFundsForTransaction.into());
    }
    // Debit from_account and credit to_account
    **from_account.try_borrow_mut_lamports()? -= amount_of_lamports;
    **to_account.try_borrow_mut_lamports()? += amount_of_lamports;
    Ok(())
}

fn transfer_service_fee_lamports(
    from_account: &AccountInfo,
    to_account: &AccountInfo,
    amount_of_lamports: u64,
) -> ProgramResult {
    // Does the from account have enough lamports to transfer?
    if **from_account.try_borrow_lamports()? < amount_of_lamports {
        return Err(CustomError::InsufficientFundsForTransaction.into());
    }
    // Debit from_account and credit to_account
    **from_account.try_borrow_mut_lamports()? -= amount_of_lamports;
    **to_account.try_borrow_mut_lamports()? += amount_of_lamports;
    Ok(())
}