use anchor_lang::{
    prelude::*,
    Discriminator,
};
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod solana_anchor {
    use super::*;

    pub fn init_lottery(
        ctx : Context<InitLottery>,
        _bump : u8,
        _ticket_price : u64,
        _start_time : u64,
        _period : u64,
        ) -> ProgramResult {
        msg!("+ init lottery");

        let pool = &mut ctx.accounts.pool;

        pool.owner = *ctx.accounts.owner.key;
        pool.rand = *ctx.accounts.rand.key;
        pool.token_mint = ctx.accounts.token_mint.key();
        pool.ticket_price = _ticket_price;
        pool.total_count = 0;
        pool.prize_mint = ctx.accounts.prize_mint.key();
        pool.start_time = _start_time;
        pool.period = _period;
        pool.ledger = *ctx.accounts.ledger.key;
        pool.win_ticket = 0;
        pool.winner = *ctx.accounts.owner.key;
        pool.closed = false;
        pool.bump = _bump;

        let mut data = (&mut ctx.accounts.ledger).data.borrow_mut();
        let mut new_data = LEDGER::discriminator().try_to_vec().unwrap();
        new_data.append(&mut pool.key().try_to_vec().unwrap());
        new_data.append(&mut (0 as u64).try_to_vec().unwrap());
        for i in 0..new_data.len(){
            data[i] = new_data[i];
        }
        let as_bytes = (MAX_LEN as u32).to_le_bytes();
        for i in 0..4{
            data[i] = as_bytes[i];
        }

        Ok(())
    }

    pub fn new_lottery (
        ctx : Context<NewLottery>,
        _ticket_price : u64,
        _start_time : u64,
        _period : u64,
        ) -> ProgramResult {
        msg!("+ new lottery");

        let pool = &mut ctx.accounts.pool;

        pool.ticket_price = _ticket_price;
        pool.total_count = 0;
        pool.prize_mint = ctx.accounts.prize_mint.key();
        pool.start_time = _start_time;
        pool.period = _period;
        pool.ledger = *ctx.accounts.ledger.key;
        pool.win_ticket = 0;
        pool.winner = *ctx.accounts.owner.key;
        pool.closed = false;

        let mut data = (&mut ctx.accounts.ledger).data.borrow_mut();
        let mut new_data = LEDGER::discriminator().try_to_vec().unwrap();
        new_data.append(&mut pool.key().try_to_vec().unwrap());
        new_data.append(&mut (0 as u64).try_to_vec().unwrap());
        for i in 0..new_data.len(){
            data[i] = new_data[i];
        }
        let as_bytes = (MAX_LEN as u32).to_le_bytes();
        for i in 0..4{
            data[i] = as_bytes[i];
        }

        Ok(())
    }

    pub fn update_lottery (
        ctx : Context<UpdateLottery>,
        _ticket_price : u64,
        _start_time : u64,
        _period : u64,
        ) -> ProgramResult {
        msg!("+ update lottery");

        let pool = &mut ctx.accounts.pool;

        pool.ticket_price = _ticket_price;
        pool.prize_mint = ctx.accounts.prize_mint.key();
        pool.start_time = _start_time;
        pool.period = _period;

        Ok(())
    }

    pub fn buy_ticket (
        ctx : Context<BuyTicket>,
        number : u64
        ) -> ProgramResult {
        msg!("+ buy ticket");

        let pool = &mut ctx.accounts.pool;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;

        if clock.unix_timestamp < pool.start_time as i64 {
            return Err(PoolError::InvalidTime.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info().clone(),
            to: ctx.accounts.pool_token_account.to_account_info().clone(),
            authority: ctx.accounts.owner.to_account_info().clone(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();
        
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, pool.ticket_price * number)?;

        for i in 0..number {
            sell_ticket(&mut ctx.accounts.ledger, (pool.total_count + i as u32) as usize, *ctx.accounts.owner.key);
        }

        pool.total_count = pool.total_count + number as u32;

        Ok(())
    }

    pub fn finish_lottery (
        ctx : Context<FinishLottery>,
        num : u32
        ) -> ProgramResult {
        msg!("+ finish lottery");
        
        let pool = &mut ctx.accounts.pool;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;

        if clock.unix_timestamp < (pool.start_time + pool.period) as i64 {
            return Err(PoolError::InvalidTime.into());
        }

        pool.win_ticket = num;
        pool.winner = get_ticket_owner(&mut ctx.accounts.ledger, num as usize)?;

        Ok(())
    }

    pub fn get_prize (
        ctx : Context<GetPrize>
        ) -> ProgramResult {
        msg!("+ get prize");

        let pool = &mut ctx.accounts.pool;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;

        if pool.closed == true {
            return Err(PoolError::InvalidTime.into());
        }

        if clock.unix_timestamp < (pool.start_time + pool.period) as i64 {
            return Err(PoolError::InvalidTime.into());
        }

        if *ctx.accounts.owner.key != pool.winner {
            return Err(PoolError::InvalidWinner.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info().clone(),
            to: ctx.accounts.user_token_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let pool_signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let pool_signer = &[&pool_signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, pool_signer);

        token::transfer(cpi_ctx, 1)?;

        pool.closed = true;

        Ok(())
    }

    pub fn emergency_withdraw(
        ctx : Context<EmergencyWithdraw>
        ) -> ProgramResult {
        msg!("+withdraw");

        let pool = &mut ctx.accounts.pool;

        if *ctx.accounts.owner.key != pool.owner {
            return Err(PoolError::InvalidOwner.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info().clone(),
            to: ctx.accounts.user_token_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let pool_signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let pool_signer = &[&pool_signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, pool_signer);

        token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn withdraw(
        ctx : Context<Withdraw>
        ) -> ProgramResult {
        msg!("+ withdraw");

        let pool = &mut ctx.accounts.pool;

        if *ctx.accounts.owner.key != pool.owner {
            return Err(PoolError::InvalidOwner.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info().clone(),
            to: ctx.accounts.user_token_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let pool_signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];
        
        let pool_signer = &[&pool_signer_seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, pool_signer);

        token::transfer(cpi_ctx, ctx.accounts.pool_token_account.amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitLottery<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(init,
        seeds = [(*rand.key).as_ref()], 
        bump = _bump, 
        payer = owner, 
        space = 8 + lottery_SIZE)]
    pool : ProgramAccount<'info, Lottery>,

    rand : AccountInfo<'info>,

    #[account(owner = spl_token::id())]
    token_mint : Account<'info, Mint>,

    #[account(mut, 
        constraint = ledger.owner == program_id)]
    ledger : AccountInfo<'info>,

    #[account(owner = spl_token::id())]
    prize_mint : Account<'info, Mint>,

    system_program : Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateLottery<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Lottery>,

    #[account(owner = spl_token::id())]
    prize_mint : Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct NewLottery<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Lottery>,

    #[account(mut, 
        constraint = ledger.owner == program_id)]
    ledger : AccountInfo<'info>,

    #[account(owner = spl_token::id())]
    prize_mint : Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct FinishLottery<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Lottery>,

    #[account(mut, 
        constraint = ledger.owner == program_id)]
    ledger : AccountInfo<'info>,

    clock : AccountInfo<'info>
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Lottery>,

    #[account(mut, 
        constraint = ledger.owner == program_id)]
    ledger : AccountInfo<'info>,

    #[account(mut,
        constraint = user_token_account.owner == owner.key(),
        constraint = user_token_account.mint == pool.token_mint)]
    user_token_account:Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_token_account.owner == pool.key(),
        constraint = pool_token_account.mint == pool.token_mint)]
    pool_token_account:Account<'info, TokenAccount>,

    clock : AccountInfo<'info>,

    token_program:Program<'info, Token>,
}

#[derive(Accounts)]
pub struct GetPrize<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Lottery>,

    #[account(mut,
        constraint = user_token_account.owner == owner.key(),
        constraint = user_token_account.mint == pool.prize_mint)]
    user_token_account:Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_token_account.owner == pool.key(),
        constraint = pool_token_account.mint == pool.prize_mint)]
    pool_token_account:Account<'info, TokenAccount>,

    clock : AccountInfo<'info>,

    token_program:Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info,Lottery>,

    #[account(mut,
        constraint = user_token_account.owner == owner.key(),
        constraint = user_token_account.mint == pool.token_mint,
        owner = spl_token::id())]
    user_token_account:Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_token_account.owner == pool.key(),
        constraint = pool_token_account.mint == pool.token_mint,
        owner = spl_token::id())]
    pool_token_account:Account<'info, TokenAccount>,

    token_program:Program<'info, Token>
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info,Lottery>,

    #[account(mut,
        constraint = user_token_account.owner == owner.key(),
        constraint = user_token_account.mint == pool.prize_mint,
        owner = spl_token::id())]
    user_token_account:Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_token_account.owner == pool.key(),
        constraint = pool_token_account.mint == pool.prize_mint,
        owner = spl_token::id())]
    pool_token_account:Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,
}

pub const LOTTERY_SIZE : usize = 32 + 32 + 32 + 32 + 8 + 4 + 8 + 8 + 32 + 4 + 32 + 1 + 1;
#[account]
pub struct Lottery {
    pub owner : Pubkey,
    pub rand : Pubkey,
    pub token_mint : Pubkey,
    pub prize_mint : Pubkey,
    pub ticket_price : u64,
    pub total_count : u32,
    pub start_time : u64,
    pub period : u64,
    pub ledger : Pubkey,
    pub win_ticket : u32,
    pub winner : Pubkey,
    pub closed : bool,
    pub bump : u8
}

pub const LEDGER_SIZE : usize = 4 + 32 * 3000;
#[account]
#[derive(Default)]
pub struct LEDGER{
    pub ledger : Vec<Pubkey>
}

pub const MAX_LEN : usize = 3000;

pub fn sell_ticket(
    a: &mut AccountInfo,
    index : usize,
    buyer : Pubkey,
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = buyer.try_to_vec().unwrap();
    let vec_start = 4 + 32 * index;
    for i in 0..data_array.len(){
        arr[vec_start+i] = data_array[i];
    }
}

pub fn get_ticket_owner(
    a : &AccountInfo,
    index : usize,
    ) -> core::result::Result<Pubkey, ProgramError> {
    let arr = a.data.borrow();
    let vec_start = 4 + 32 * index;
    let data_array = &arr[vec_start..vec_start+32];
    let owner : Pubkey = Pubkey::try_from_slice(data_array)?;
    Ok(owner)
}

#[error]
pub enum PoolError {
    #[msg("Token mint to failed")]
    TokenMintToFailed,

    #[msg("Token set authority failed")]
    TokenSetAuthorityFailed,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Token burn failed")]
    TokenBurnFailed,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid time")]
    InvalidTime,

    #[msg("Invalid pool ledger")]
    InvalidPoolLedger,

    #[msg("Invalid period")]
    InvalidPeriod,

    #[msg("Invalid owner")]
    InvalidOwner,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid stake amount")]
    InvalidStakeAmount,

    #[msg("Invalid winner")]
    InvalidWinner,
}