use anchor_lang::prelude::*;
use anchor_lang::{solana_program, system_program};
use anchor_spl::associated_token::{self, AssociatedToken};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj");

fn get_hash(timestamp: i64, nonce: u64, sender: Pubkey) -> Result<solana_program::keccak::Hash> {
    let serialized = (timestamp, nonce, sender).try_to_vec()?;
    let hash = solana_program::keccak::hash(&serialized[..]);

    Ok(hash)
}

#[program]
mod solana_cdt_bridge {
    use anchor_lang::system_program::{create_account, CreateAccount};
    use solana_program::borsh0_10::try_from_slice_unchecked;

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        chain: String,
        fees_in_dollar: u64,
        fees_in_cdt_percentage: u64,
    ) -> Result<()> {
        let bridge_info = &mut ctx.accounts.bridge_info;
        let bridge_info_bump = ctx.bumps.bridge_info;
        let token_vaults_bump = ctx.bumps.token_vaults;
        let native_vaults_bump = ctx.bumps.native_vaults;

        create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.native_vaults.to_account_info(),
                },
                &[&[b"bridge_native_vaults", &[native_vaults_bump]]],
            ),
            Rent::get()?.minimum_balance(8),
            8 as u64,
            ctx.program_id,
        )?;

        bridge_info.token = ctx.accounts.token_mint.key();
        bridge_info.chain = chain;
        bridge_info.owner = *ctx.accounts.authority.key;
        bridge_info.program = *ctx.accounts.authority.key;
        bridge_info.fees_in_dollar = fees_in_dollar;
        bridge_info.fees_in_cdt_percentage = fees_in_cdt_percentage;
        bridge_info.minimum_transfer_quantity = 0;
        bridge_info.bridge_fees_in_cdt = 0;
        bridge_info.lock_ask_duration = 86400 * 2;
        bridge_info.unlock_ask_duration = 84600 * 15;
        bridge_info.unlock_ask_time = 0;
        bridge_info.transfers_length = 0;
        bridge_info.dex_in = ctx.accounts.dex_in_token_mint.key();
        bridge_info.dex_out = ctx.accounts.dex_out_token_mint.key();
        bridge_info.dex_pool = *ctx.accounts.dex_pool.key;
        bridge_info.paused = false;
        bridge_info.bump = bridge_info_bump;
        bridge_info.token_vaults_bump = token_vaults_bump;
        bridge_info.native_vaults_bump = native_vaults_bump;

        Ok(())
    }

    pub fn set_fees_in_dollar(ctx: Context<AdminAction>, fees_in_dollar: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.fees_in_dollar = fees_in_dollar;

        Ok(())
    }

    pub fn set_fees_in_cdt_percentage(
        ctx: Context<AdminAction>,
        fees_in_cdt_percentage: u64,
    ) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.fees_in_cdt_percentage = fees_in_cdt_percentage;

        Ok(())
    }

    pub fn ask_withdraw(ctx: Context<AdminAction>) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        let clock = Clock::get()?;
        ctx.accounts.bridge_info.unlock_ask_time = clock.unix_timestamp as u64;
        Ok(())
    }

    pub fn set_paused(ctx: Context<AdminAction>, paused: bool) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.paused = paused;
        Ok(())
    }

    pub fn set_minimum_transfer_quantity(ctx: Context<AdminAction>, quantity: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.minimum_transfer_quantity = quantity;
        Ok(())
    }

    pub fn update_transfer_cost(ctx: Context<AdminAction>, fees_in_dollar: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.fees_in_dollar = fees_in_dollar;
        Ok(())
    }

    pub fn change_owner(ctx: Context<ChangeOwner>) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.owner = *ctx.accounts.owner.key;
        Ok(())
    }

    pub fn change_program(ctx: Context<ChangeProgram>) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        ctx.accounts.bridge_info.program = *ctx.accounts.program.key;
        Ok(())
    }

    pub fn set_dex(ctx: Context<SetDex>) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }

        ctx.accounts.bridge_info.dex_in = ctx.accounts.dex_in_token_mint.key();
        ctx.accounts.bridge_info.dex_out = ctx.accounts.dex_out_token_mint.key();
        ctx.accounts.bridge_info.dex_pool = *ctx.accounts.dex_pool.key;

        Ok(())
    }

    pub fn collect_cdt_fees(ctx: Context<CollectCDTFees>) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        if ctx.accounts.token_vaults.amount < ctx.accounts.bridge_info.bridge_fees_in_cdt {
            return err!(BridgeError::InsufficientBalance);
        }

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vaults.to_account_info(),
                    to: ctx.accounts.receiver_token.to_account_info(),
                    authority: ctx.accounts.token_vaults.to_account_info(),
                },
                &[&[
                    b"bridge_token_vaults",
                    ctx.accounts.bridge_info.token.as_ref(),
                    &[ctx.accounts.bridge_info.token_vaults_bump],
                ]],
            ),
            ctx.accounts.bridge_info.bridge_fees_in_cdt,
        )?;

        ctx.accounts.bridge_info.bridge_fees_in_cdt = 0;

        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, quantity: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        if ctx.accounts.sender_token.amount < quantity {
            return err!(BridgeError::InsufficientBalance);
        }

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token.to_account_info(),
                    to: ctx.accounts.token_vaults.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            quantity,
        )?;

        Ok(())
    }

    pub fn deposit_sol(ctx: Context<DepositSOL>, quantity: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        if ctx.accounts.authority.to_account_info().lamports() < quantity {
            return err!(BridgeError::InsufficientBalance);
        }

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.native_vaults.to_account_info(),
                },
            ),
            quantity,
        )?;

        Ok(())
    }

    pub fn withdraw_sol(ctx: Context<WithdrawSOL>, quantity: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        if ctx.accounts.native_vaults.to_account_info().lamports() < quantity {
            return err!(BridgeError::InsufficientBalance);
        }

        ctx.accounts.native_vaults.sub_lamports(quantity)?;
        ctx.accounts.receiver.add_lamports(quantity)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, quantity: u64) -> Result<()> {
        if ctx.accounts.authority.key() != ctx.accounts.bridge_info.owner {
            return err!(BridgeError::NotOwner);
        }
        let timestamp = Clock::get()?.unix_timestamp as u64;
        if ctx.accounts.bridge_info.unlock_ask_time
            >= timestamp - ctx.accounts.bridge_info.lock_ask_duration
        {
            return err!(BridgeError::MinimumLockedPeriod);
        }
        if ctx.accounts.bridge_info.unlock_ask_time
            <= timestamp - ctx.accounts.bridge_info.unlock_ask_duration
        {
            return err!(BridgeError::MaximumUnlockedPeriod);
        }
        if ctx.accounts.token_vaults.amount < quantity {
            return err!(BridgeError::InsufficientBalance);
        }

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vaults.to_account_info(),
                    to: ctx.accounts.receiver_token.to_account_info(),
                    authority: ctx.accounts.token_vaults.to_account_info(),
                },
                &[&[
                    b"bridge_token_vaults",
                    ctx.accounts.bridge_info.token.as_ref(),
                    &[ctx.accounts.bridge_info.token_vaults_bump],
                ]],
            ),
            quantity,
        )?;

        Ok(())
    }

    pub fn init_transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, InitTransfer<'info>>,
        quantity: u64,
        chain: String,
        data: String,
    ) -> Result<()> {
        let bridge_info = &mut ctx.accounts.bridge_info;

        if quantity < bridge_info.minimum_transfer_quantity {
            return err!(BridgeError::InsufficientQuantity);
        }
        if ctx.accounts.sender_token.amount < quantity {
            return err!(BridgeError::InsufficientBalance);
        }
        if bridge_info.paused == true {
            return err!(BridgeError::NotActived);
        }

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token.to_account_info(),
                    to: ctx.accounts.token_vaults.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            quantity,
        )?;

        let one_dollar =
            ctx.accounts.pool_in_token.amount * 1_000_000_000 / ctx.accounts.pool_out_token.amount;

        let transfer_fees_in_cdt = quantity * bridge_info.fees_in_cdt_percentage / 100;
        let transfer_quantity = quantity - transfer_fees_in_cdt;
        let transfer_sol_fees = one_dollar * 1_000_000_000 / bridge_info.fees_in_dollar * 100;

        if ctx.accounts.authority.to_account_info().lamports() < transfer_sol_fees {
            return err!(BridgeError::InsufficientBalance);
        }

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.token_vaults.to_account_info(),
                },
            ),
            transfer_sol_fees,
        )?;

        bridge_info.bridge_fees_in_cdt += transfer_fees_in_cdt;

        let transfer_hash = get_hash(
            Clock::get()?.unix_timestamp,
            0,
            ctx.accounts.authority.key(),
        )?
        .to_string();

        let (_transfer_pda, bump) = Pubkey::find_program_address(
            &[
                b"bridge_transfer",
                bridge_info.transfers_length.to_le_bytes().as_ref(),
            ],
            &ctx.program_id.key(),
        );

        let space = 8 + std::mem::size_of::<TransferInfo>();

        create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.bridge_transfer.to_account_info(),
                },
                &[&[
                    b"bridge_transfer",
                    bridge_info.transfers_length.to_le_bytes().as_ref(),
                    &[bump],
                ]],
            ),
            Rent::get()?.minimum_balance(space),
            space as u64,
            ctx.program_id,
        )?;

        let mut transfer_account_data =
            try_from_slice_unchecked::<TransferInfo>(&ctx.accounts.bridge_transfer.data.borrow())
                .unwrap();

        transfer_account_data.hash = transfer_hash;
        transfer_account_data.from = *ctx.accounts.authority.key;
        transfer_account_data.coin = bridge_info.token.clone();
        transfer_account_data.quantity = transfer_quantity;
        transfer_account_data.from_chain = bridge_info.chain.clone();
        transfer_account_data.to_chain = chain;
        transfer_account_data.fees_in_cdt = transfer_fees_in_cdt;
        transfer_account_data.fees_in_sol = transfer_sol_fees;
        transfer_account_data.block_timestamp = Clock::get()?.unix_timestamp as u64;
        transfer_account_data.block_number = Clock::get()?.slot;
        transfer_account_data.data = data;

        transfer_account_data
            .serialize(&mut &mut ctx.accounts.bridge_transfer.data.borrow_mut()[..])?;

        bridge_info.transfers_length += 1;

        Ok(())
    }

    pub fn add_transfers_from<'info>(
        ctx: Context<'_, '_, '_, 'info, AddTransfersFrom<'info>>,
        _chains: Vec<String>,
        amounts: Vec<u64>,
        _transfers_hashs: Vec<String>,
    ) -> Result<()> {
        let bridge_info = &mut ctx.accounts.bridge_info;

        if ctx.accounts.authority.key() != bridge_info.owner
            && ctx.accounts.authority.key() != bridge_info.program
        {
            return err!(BridgeError::NotOwnerProgram);
        }

        let len = amounts.len();

        for i in 0..len {
            let account = ctx.remaining_accounts.get(i * 2).unwrap().clone();
            let account_ata = ctx.remaining_accounts.get(i * 2 + 1).unwrap().clone();
            let amount = *amounts.get(i).unwrap();
            let account_data_len = account_ata.data.borrow().len();
            msg!("TEST: {:?} {:?}", account.key(), account_data_len);
            if account_data_len == 0 {
                associated_token::create(CpiContext::new(
                    ctx.accounts.associated_token_program.to_account_info(),
                    associated_token::Create {
                        associated_token: account_ata.clone(),
                        mint: ctx.accounts.token_mint.to_account_info(),
                        authority: account.clone(),
                        payer: ctx.accounts.authority.to_account_info(),
                        token_program: ctx.accounts.token_program.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                    },
                ))?;
            }

            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.token_vaults.to_account_info(),
                        to: account_ata.clone(),
                        authority: ctx.accounts.token_vaults.to_account_info(),
                    },
                    &[&[
                        b"bridge_token_vaults",
                        ctx.accounts.bridge_info.token.as_ref(),
                        &[ctx.accounts.bridge_info.token_vaults_bump],
                    ]],
                ),
                amount,
            )?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, seeds = [b"bridge_info"], space = 10000, bump)]
    pub bridge_info: Account<'info, BridgeInfo>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    #[account(init, payer = authority, seeds = [b"bridge_token_vaults", token_mint.key().as_ref()], bump, token::mint = token_mint, token::authority = token_vaults)]
    pub token_vaults: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, seeds = [b"bridge_native_vaults"], bump)]
    pub native_vaults: SystemAccount<'info>,

    #[account(mut)]
    pub dex_in_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub dex_out_token_mint: Account<'info, Mint>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub dex_pool: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ChangeOwner<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ChangeProgram<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub program: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetDex<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut)]
    pub dex_in_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub dex_out_token_mint: Account<'info, Mint>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub dex_pool: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CollectCDTFees<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut, token::mint = bridge_info.token)]
    pub receiver_token: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"bridge_token_vaults", bridge_info.token.as_ref()], bump = bridge_info.token_vaults_bump)]
    pub token_vaults: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut, token::mint = bridge_info.token)]
    pub sender_token: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"bridge_token_vaults", bridge_info.token.as_ref()], bump = bridge_info.token_vaults_bump)]
    pub token_vaults: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, seeds = [b"bridge_native_vaults"], bump=bridge_info.native_vaults_bump)]
    pub native_vaults: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub receiver: AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, seeds = [b"bridge_native_vaults"], bump=bridge_info.native_vaults_bump)]
    pub native_vaults: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut, token::mint = bridge_info.token)]
    pub receiver_token: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"bridge_token_vaults", bridge_info.token.as_ref()], bump = bridge_info.token_vaults_bump)]
    pub token_vaults: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AddTransfersFrom<'info> {
    #[account(mut, seeds = [b"bridge_info"], bump = bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut, seeds = [b"bridge_token_vaults", bridge_info.token.as_ref()], bump = bridge_info.token_vaults_bump)]
    pub token_vaults: Account<'info, TokenAccount>,

    #[account(mut, address = bridge_info.token)]
    pub token_mint: Account<'info, Mint>,

    #[account(mut, token::mint = bridge_info.token, token::authority = authority)]
    pub sender_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitTransfer<'info> {
    #[account(mut, seeds=[b"bridge_info"], bump=bridge_info.bump)]
    pub bridge_info: Account<'info, BridgeInfo>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, token::mint = bridge_info.token)]
    pub sender_token: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"bridge_token_vaults", bridge_info.token.as_ref()], bump = bridge_info.token_vaults_bump)]
    pub token_vaults: Account<'info, TokenAccount>,

    #[account(mut, token::mint = bridge_info.dex_in, token::authority = bridge_info.dex_pool)]
    pub pool_in_token: Account<'info, TokenAccount>,
    #[account(mut, token::mint = bridge_info.dex_out, token::authority = bridge_info.dex_pool)]
    pub pool_out_token: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, seeds=[b"bridge_transfer", bridge_info.transfers_length.to_le_bytes().as_ref()], bump)]
    pub bridge_transfer: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct BridgeInfo {
    pub token: Pubkey,
    pub owner: Pubkey,
    pub program: Pubkey,
    pub chain: String,

    pub fees_in_dollar: u64,
    pub fees_in_cdt_percentage: u64,
    pub minimum_transfer_quantity: u64,

    pub bridge_fees_in_cdt: u64,
    pub lock_ask_duration: u64,
    pub unlock_ask_duration: u64,
    pub unlock_ask_time: u64,

    pub transfers_length: u64,
    pub dex_in: Pubkey,
    pub dex_out: Pubkey,
    pub dex_pool: Pubkey,

    pub paused: bool,
    bump: u8,
    token_vaults_bump: u8,
    native_vaults_bump: u8,
}

#[account]
#[derive(Debug)]
pub struct TransferInfo {
    pub hash: String,
    pub from: Pubkey,
    pub coin: Pubkey,
    pub quantity: u64,
    pub from_chain: String,
    pub to_chain: String,
    pub fees_in_cdt: u64,
    pub fees_in_sol: u64,
    pub block_timestamp: u64,
    pub block_number: u64,
    pub data: String,
}

#[error_code]
pub enum BridgeError {
    #[msg("INSUFFICIENT_QUANTITY")]
    InsufficientQuantity,
    #[msg("INSUFFICIENT_BALANCE")]
    InsufficientBalance,
    #[msg("Out of bounds")]
    OutofBounds,
    #[msg("PAYMENT_ABORT")]
    PaymentAbort,
    #[msg("Transfer Failed!")]
    TransferFailed,
    #[msg("2_DAYS_MINIMUM_LOCKED_PERIOD")]
    MinimumLockedPeriod,
    #[msg("15_DAYS_MAXIMUM_UNLOCKED_PERIOD")]
    MaximumUnlockedPeriod,
    #[msg("Already transfered")]
    AlreadyTransfered,
    #[msg("Only the owner can do this action")]
    NotOwner,
    #[msg("Only program or Owner")]
    NotOwnerProgram,
    #[msg("Bridge actually paused")]
    NotActived,
}
