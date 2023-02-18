use anchor_lang::{
    prelude::*,
    Discriminator,
};
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use crate::utils::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod solana_anchor {
    use super::*;

    pub fn init_pool(
        ctx : Context<InitPool>,
        _bump : u8,
        _fee : u64,
        ) -> ProgramResult {
        msg!("+ init_pool");

        let pool = &mut ctx.accounts.pool;

        pool.owner = *ctx.accounts.owner.key;
        pool.rand = *ctx.accounts.rand.key;
        pool.fee_receiver = ctx.accounts.fee_receiver.key();
        pool.winner = ctx.accounts.winner.key();
        pool.fee = _fee;
        pool.bump = _bump;

        Ok(())
    }

    pub fn start_round (
        ctx: Context<StartRound>,
        _round_name : String,
        _total_ticket : u64,
        _round_period : u64,
    ) -> ProgramResult {
        msg!("+ start new round");

        let round_data = &mut ctx.accounts.round_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;

        round_data.pool = ctx.accounts.pool.key();
        round_data.ticket_ledger = *ctx.accounts.ticket_ledger.key;
        round_data.total_ticket = _total_ticket;
        round_data.start_time = clock.unix_timestamp as u64;
        round_data.round_period = _round_period;
        round_data.tvl = 0;
        round_data.claimed = false;
        round_data.finished = false;
        round_data.round_name = _round_name;
        round_data.bump = _bump;

        let mut data = (&mut ctx.accounts.ticket_ledger).data.borrow_mut();
        let mut new_data = TicketList::discriminator().try_to_vec().unwrap();

        new_data.append(&mut round_data.key().try_to_vec().unwrap());
        new_data.append(&mut (0 as u32).try_to_vec().unwrap());

        for i in 0..bew_data.len() {
            data[i] = new_data[i];
        }

        let vec_start = 8 + 32 + 4;
        let as_bytes = (_total_ticket as u32).to_le_bytes();

        for i in 0..4 {
            data[vec_start + i] = as_bytes[i];
        }

        Ok(())
    }

    pub fn finish_round (
        ctx: Context<FinishRound>
    ) -> ProgramResult {
        msg!("+ finish current round");

        let round = &mut ctx.accounts.round;

        // Generate a random number
        let recent_slothashes = &ctx.accounts.recent_blockhashes;
        if cmp_pubkeys(&recent_slothashes.key(), &BLOCK_HASHES) {
            msg!("recent_blockhashes is deprecated and will break soon");
        }
        if !cmp_pubkeys(&recent_slothashes.key(), &SlotHashes::id())
            && !cmp_pubkeys(&recent_slothashes.key(), &BLOCK_HASHES)
        {
            return err!(PoolError::IncorrectSlotHashesPubkey);
        }

        let data = recent_slothashes.data.borrow();
        let most_recent = array_ref![data, 12, 8];

        let winner_index = u64::from_le_bytes(*most_recent);
        /////////////////////////////

        let winner_ticket = get_winning_ticket(&ctx.accounts.ticket_ledger, winner_index as usize)?;

        round.winner = winner_ticket.owner;
        round.finished = true;

        Ok(())
    }

    pub buy_ticket (
        ctx : Context<BuyTicket>,
    ) -> ProgramResult {
        msg!("+ buy ticket");

        let pool = &mut ctx.accounts.pool;
        let round = &mut ctx.accounts.round;

        let last_number = get_last_number(&ctx.accounts.ticket_ledger)?;

        if last_number > round.total_ticket - 1 {
            return Err(PoolError::TicketLimitReached.into());
        }

        sol_transfer_without_seed(
            SolTransferParamsWithoutSeed {
                source: ctx.accounts.owner.clone(),
                destination: pool.to_account_info().clone(),
                system_program: ctx.accounts.system_program.to_account_info().clone(),
                amount: 0.25 * 1_000_000_000,
            }
        )?;

        sol_transfer_without_seed(
            SolTransferParamsWithoutSeed {
                source: ctx.accounts.owner.clone(),
                destination: ctx.accounts.fee_receiver.clone(),
                system_program: ctx.accounts.system_program.to_account_info().clone(),
                amount: 0.02 * 1_000_000_000,
            }
        )?;

        set_ticket_owner(
            &mut ctx.accounts.ticket_ledger, 
            last_number as usize, 
            TicketData {
                ticket_index : (last_number + 1) as u64,
                owner : *ctx.accounts.owner.key
            }
        );

        set_last_number(&mut ctx.accounts.ticket_ledger, last_number + 1);

        round.tvl += 0.25 * 1_000_000_000;

        Ok(())
    }

    pub fn claim {
        ctx : Context<Claim>
    } -> ProgramResult {
        msg!("+ claim");

        let pool = &mut ctx.accounts.pool;
        let round = &mut ctx.accounts.round;

        if !round.finished {
            return Err(PoolError::RoundNotFinished.into());
        }

        if *ctx.accounts.owner.key != round.winner && *ctx.accounts.owner.key != pool.winner {
            return Err(PoolError::InvalidWInner.into());
        }

        sol_transfer(
            SolTransferParams {
                source: pool.to_account_info().clone(),
                destination: ctx.accounts.owner.clone(),
                amount: round.tvl,
            }
        )?;

        round.claimed = true;
        round.tvl = 0;

        Ok(())
    }

    pub fn withdraw {
        ctx : Context<WithDraw>,
        _amount : u64
    } -> ProgramResult {
        msg!("+ withdraw");

        let pool = &mut ctx.accounts.pool;
        let round = &mut ctx.accounts.round;

        sol_transfer(
            SolTransferParams {
                source: pool.to_account_info().clone(),
                destination: ctx.accounts.owner.clone(),
                amount: _amount,
            }
        )?;
        
        round.tvl -= _amount;

        Ok(())
    }

    pub fn deposit {
        ctx : Context<Deposit>,
        _amount : u64
    } -> ProgramResult {
        msg!("+ deposit");

        let pool = &mut ctx.accounts.pool;
        let round = &mut ctx.accounts.round;

        sol_transfer_without_seed(
            SolTransferParamsWithoutSeed {
                source: ctx.accounts.owner.clone(),
                destination: pool.to_account_info().clone(),
                system_program: ctx.accounts.system_program.to_account_info().clone(),
                amount: _amount,
            }
        )?;
        
        round.tvl += _amount;

        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitPool<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(init,
        seeds = [(*rand.key).as_ref()], 
        bump = _bump, 
        payer = owner, 
        space = 8 + POOL_SIZE)]
    pool : ProgramAccount<'info, Pool>,

    rand : AccountInfo<'info>,

    fee_receiver : AccountInfo<'info>,

    winner : AccountInfo<'info>,

    system_program : Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_bump : u8, _round_name: String)]
pub struct StartRound<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut)]
    ticket_ledger : AccountInfo<'info>,

    #[account(init, 
        seeds = [pool.key().as_ref(), _round_name.as_ref()], 
        bump = _bump, 
        payer = owner, 
        space = 8 + ROUND_SIZE)]
    round_data : ProgramAccount<'info, Round>,

    clock : AccountInfo<'info>,

    system_program : Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinishRound<'info>{
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut)]
    ticket_ledger : AccountInfo<'info>,

    #[account(mut,
        has_one = pool,
        has_one = ticket_ledger,
        seeds = [round.pool.key().as_ref(), round.round_name.as_ref()], 
        bump = round.bump)]
    round : ProgramAccount<'info, Round>,

    /// CHECK: checked in program.
    recent_blockhashes: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = fee_receiver)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut)]
    fee_receiver : AccountInfo<'info>,

    #[account(mut,
        has_one = pool,
        has_one = ticket_ledger,
        seeds = [round.pool.key().as_ref(), round.round_name.as_ref()], 
        bump = round.bump)]
    round : ProgramAccount<'info, Round>,

    #[account(mut)]
    ticket_ledger : AccountInfo<'info>,

    system_program : Program<'info, System>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        has_one = pool,
        seeds = [round.pool.key().as_ref(), round.round_name.as_ref()], 
        bump = round.bump)]
    round : ProgramAccount<'info, Round>
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        has_one = pool,
        seeds = [round.pool.key().as_ref(), round.round_name.as_ref()], 
        bump = round.bump)]
    round : ProgramAccount<'info, Round>
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        has_one = pool,
        seeds = [round.pool.key().as_ref(), round.round_name.as_ref()], 
        bump = round.bump)]
    round : ProgramAccount<'info, Round>,

    system_program : Program<'info, System>,
}

pub const POOL_SIZE : usize = 32 + 32 + 32 + 32 + 8 + 1;
#[account]
pub struct Pool {
    pub owner : Pubkey,
    pub rand : Pubkey,
    pub fee_receiver : Pubkey,
    pub winner : Pubkey,
    pub fee : u64,
    pub bump : u8,
}

pub const ROUND_SIZE : usize = 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 4 + 10 + 1;
#[account]
pub struct Round {
    pub pool : Pubkey,
    pub ticket_ledger : Pubkey,
    pub winner : Pubkey,
    pub total_ticket : u64,
    pub start_time : u64,
    pub round_period : u64,
    pub tvl : u64,
    pub claimed : bool,
    pub finished : bool,
    pub round_name : String,
    pub bump : u8
}

pub const MAX_LEN : usize = 10000;
pub const POOL_LEDGER_SIZE : usize = 32 + 4 + TICKET_DATA_SIZE * MAX_LEN;
#[account]
#[derive(Default)]
pub struct TicketList {
    pub round : Pubkey,
    pub last_number : u32,
    pub ticket_ledger : Vec<TicketData>
}

pub const TICKET_DATA_SIZE : usize = 8 + 32;
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TicketData{
    pub ticket_index : u64,
    pub owner : Pubkey,
}

pub fn set_ticket_owner(
    a: &mut AccountInfo,
    index : usize,
    ticket_data : TicketData,
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = ticket_data.try_to_vec().unwrap();
    let vec_start = 8 + 32 + 4 + TICKET_DATA_SIZE * index;
    for i in 0..data_array.len(){
        arr[vec_start+i] = data_array[i];
    }
}

pub fn set_last_number(
    a: &mut AccountInfo,
    number : u32,
    ){
    let mut arr = a.data.borrow_mut();
    let data_array = number.try_to_vec().unwrap();
    let vec_start = 40;
    for i in 0..data_array.len() {
        arr[vec_start+i] = data_array[i];
    }    
}

pub fn get_winning_ticket(
    a : &AccountInfo,
    index : usize,
    ) -> core::result::Result<TicketData, ProgramError> {
    let arr = a.data.borrow();
    let vec_start = 8 + 32 + 4 + TICKET_DATA_SIZE * index;
    let data_array = &arr[vec_start..vec_start+TICKET_DATA_SIZE];
    let ticket_data : TicketData = TicketData::try_from_slice(data_array)?;
    Ok(ticket_data)
}

pub fn get_last_number(
    a : &AccountInfo
    ) -> core::result::Result<u32, ProgramError>{
    let arr= a.data.borrow();
    let data_array = &arr[40..44];
    let last_number : u32 = u32::try_from_slice(data_array)?;
    Ok(last_number)
}

#[error]
pub enum PoolError {
    #[msg("Current round is not finihsed yet")]
    RoundNotFinished,

    #[msg("All tickets are already sold")]
    TicketLimitReached,

    #[msg("Token set authority failed")]
    TokenSetAuthorityFailed,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Token burn failed")]
    TokenBurnFailed,

    #[msg("Invalid Ranking")]
    InvalidRanking,

    #[msg("Invalid time")]
    InvalidTime,

    #[msg("Invalid pool ledger")]
    InvalidPoolLedger,

    #[msg("Invalid period")]
    InvalidPeriod,

    #[msg("Invalid metadata extended account")]
    InvalidMetadataExtended,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid winner")]
    InvalidWinner,

    #[msg("Invalid withdraw amount")]
    InvalidWithdrawAmount,

    #[msg("Incorrect collection NFT authority")]
    IncorrectSlotHashesPubkey,
}