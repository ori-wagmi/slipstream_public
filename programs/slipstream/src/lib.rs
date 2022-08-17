use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("EC24e33sBdiJsWWDhYU6qVRJEji4s2y52TAj2PPLp25o");

#[program]
pub mod vesting {
    use super::*;

    const STAKE_PDA_SEED: &[u8] = b"stakeVault";

    // Create vault with initial values
    pub fn create_stake_vault(ctx: Context<CreateStakeVault>, _bump: u8, length: i64) -> Result<()> {
        msg!("Create vault function got called: {}",ctx.accounts.signer.key());

        let (_pda, _bump_seed)  = Pubkey::find_program_address(&[STAKE_PDA_SEED], ctx.program_id);
        let clock:Clock = Clock::get().unwrap();
        ctx.accounts.vault_account.bump_seed = _bump_seed;
        ctx.accounts.vault_account.multisig = *ctx.accounts.multisig.key;
        ctx.accounts.vault_account.vest_start_time = clock.unix_timestamp;
        ctx.accounts.vault_account.vest_length = length;
        ctx.accounts.vault_account.frozen = false;
        Ok(())
    }

    // deposits `amount` of SOL into the vault
    pub fn deposit_into_vault(ctx: Context<Deposit>, amount: u64, ) -> Result<()> {
        let user = & ctx.accounts.signer;
        let from =user.to_account_info();
        let to = ctx.accounts.vault.to_account_info();

        //sending sols
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from,
                to,
            });
        system_program::transfer(cpi_context, amount)?;
        Ok(())
    }

    // queries vested SOL from vault
    pub fn vault_pending_claim(ctx: Context<Claim>) -> Result<i64> {
        let vault =  &mut ctx.accounts.vault;
        let vault_account_info = vault.to_account_info();
        let vault_lamports = **vault_account_info.try_borrow_lamports().unwrap() as i64;
        let current_timestamp = Clock::get().unwrap().unix_timestamp;

        if current_timestamp < vault.vest_start_time {
            return Err(ErrorCode::VestingNotStarted.into());
        }

        let pending_sol: i64;
        if current_timestamp < (vault.vest_start_time + vault.vest_length) {
            // (currentTime - vestingStartTime) / vestingLength * depositedSOL
            pending_sol = (current_timestamp - vault.vest_start_time) * vault_lamports / vault.vest_length; // this does integer arithmtic, pls fix.
        }
        else {
            pending_sol = vault_lamports;
        }

        Ok(pending_sol)
    }

    pub fn claim_vested_sol(ctx: Context<Claim>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        if vault.frozen {
            return Err(ErrorCode::FundsFrozen.into());
        }

        /*******************************************
        this needs to be replaced with TeamWallet allocation
        ********************************************/
        let from = vault.to_account_info();
        let to = ctx.accounts.signer.to_account_info();
        let lamports = 10; /*  vault_pending_claim(ctx).unwrap() as u64; // need to move ctx into this function */

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from,
                to,
            });
        system_program::transfer(cpi_context, lamports)?;
        Ok(())
    }

    // sets `frozen` on vault, only callable by multisig.
    pub fn freeze_funds(ctx: Context<Claim>, frozen: bool) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let multisig_key = vault.multisig;
        let signer_key = ctx.accounts.signer.to_account_info().key;
        if multisig_key != *signer_key {
            return Err(ErrorCode::NotMultisig.into());
        }

        vault.frozen = frozen;
        Ok(())
    }

    // refunds all SOL from vault to multisig.
    // Only callable by multisig, only callable when frozen.
    pub fn initiate_refund(ctx: Context<Claim>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        if !vault.frozen {
            return Err(ErrorCode::FundsNotFrozen.into());
        }

        let multisig_key = vault.multisig;
        let to = ctx.accounts.signer.to_account_info();
        if multisig_key != *(to.key) {
            return Err(ErrorCode::NotMultisig.into());
        }

        let from = vault.to_account_info();
        let lamports = **(from.try_borrow_lamports().unwrap());

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from,
                to,
            });
        system_program::transfer(cpi_context, lamports)?;
        Ok(())
    }
}


#[account]
pub struct StakeVault{
    pub bump_seed:u8,
    pub multisig:Pubkey,
    pub vest_start_time:i64,
    pub vest_length:i64,
    pub frozen:bool,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateStakeVault<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub multisig: AccountInfo<'info>,
    #[account(init,
    seeds = [b"stakeVault".as_ref()], bump,
    payer = signer, space = 8 + 64 + 64 + 40 + 32 )]
    pub vault_account: Account<'info, StakeVault>,
    pub rent: Sysvar<'info,Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// CHECK:
    #[account(mut)]
    pub signer: AccountInfo<'info>,
    /// CHECK:
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub vault: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, StakeVault>,
    /// CHECK:
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Super Owner")]
    InvalidSuperOwner,

    #[msg("Tokens already staked.")]
    AlreadyStaked,

    #[msg("Tokens already un-staked.")]
    AlreadyUnStaked,

    #[msg("Invalid Withdraw Time")]
    InvalidWithdrawTime,

    #[msg("Insufficient Reward Token Balance")]
    InsufficientRewardVault,

    #[msg("Vesting hasn't started")]
    VestingNotStarted,

    #[msg("Not called by multisig")]
    NotMultisig,

    #[msg("funds are frozen")]
    FundsFrozen,

    #[msg("funds not frozen")]
    FundsNotFrozen,
}