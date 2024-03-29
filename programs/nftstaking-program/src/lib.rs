use anchor_lang::prelude::*;

declare_id!("E6vWq2ax8LG1hVShtFR3hNKAmZFAYVmeokwseAfi63ic");

#[program]
pub mod nftstaking_program {
    use {
        anchor_lang::AccountsClose,
        anchor_spl::token::{mint_to, approve, revoke},
        mpl_token_metadata::{
            instruction::{
                create_metadata_accounts_v3, 
                create_master_edition_v3, 
                freeze_delegated_account, 
                thaw_delegated_account
            }, 
            state::Creator
        },
        solana_program::program::{invoke, invoke_signed},
        super::*,
    };

    // Initialize the mint
    pub fn initialize_mint(ctx: Context<InitializeMint>) -> Result<()> {
        msg!("Created token {}", ctx.accounts.token_mint.key());
        Ok(())
    }

    // Create and mint NFT
    pub fn create_nft(ctx: Context<CreateNFT>, name: String, symbol: String, uri: String) -> Result<()> {
        // Define NFT creators
        let creators = Some(vec![
            Creator{ 
                address: ctx.accounts.user.key(), 
                verified: true, 
                share: 100 
            }
        ]);

        // Create metadata account
        let create_metadata_instruction = create_metadata_accounts_v3(
            ctx.accounts.metadata_program.key(), 
            ctx.accounts.metadata_account.key(), 
            ctx.accounts.nft_mint.key(), 
            ctx.accounts.user.key(), 
            ctx.accounts.user.key(), 
            ctx.accounts.user.key(), 
            name, 
            symbol, 
            uri, 
            creators, 
            0, 
            false, 
            false, 
            None, 
            None, 
            None
        );

        // Submit the instruction
        invoke(
            &create_metadata_instruction, 
            &[
                ctx.accounts.metadata_program.to_account_info(),
                ctx.accounts.metadata_account.to_account_info(),
                ctx.accounts.nft_mint.to_account_info(),
                ctx.accounts.user.to_account_info(),
            ]
        )?;

        // Mint the NFT
        mint_to(ctx.accounts.mint_to_ctx(), 1)?;

        // Create master edition
        let create_master_edition_ix = create_master_edition_v3(
            ctx.accounts.metadata_program.key(), 
            ctx.accounts.master_edition.key(), 
            ctx.accounts.nft_mint.key(), 
            ctx.accounts.user.key(), 
            ctx.accounts.user.key(), 
            ctx.accounts.metadata_account.key(), 
            ctx.accounts.user.key(), 
            Some(1)
        );

        // Submit the instruction
        invoke(
            &create_master_edition_ix, 
            &[
                ctx.accounts.metadata_program.to_account_info(),
                ctx.accounts.master_edition.to_account_info(),
                ctx.accounts.nft_mint.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.metadata_account.to_account_info(),
            ]
        )?;

        Ok(())
    }

    // Stake NFT
    pub fn stake(ctx: Context<StakeNFT>) -> Result<()> {
        // Approve the staking operation
        approve(ctx.accounts.approve_ctx(), 1)?;

        // Prepare authority for invoking freeze
        let authority_bump = *ctx.bumps.get("program_authority").unwrap();
        let authority_seeds = &["authority".as_bytes(), &[authority_bump]];
        let signer = &[&authority_seeds[..]];

        // Freeze the account
        let freeze_ix = freeze_delegated_account(
            ctx.accounts.metadata_program.key(), 
            ctx.accounts.program_authority.key(), 
            ctx.accounts.user_token_account.key(), 
            ctx.accounts.master_edition.key(), 
            ctx.accounts.nft_mint.key()
        );

        // Invoke freeze with signature
        invoke_signed(
            &freeze_ix, 
            &[
                ctx.accounts.metadata_program.to_account_info(),
                ctx.accounts.program_authority.to_account_info(),
                ctx.accounts.user_token_account.to_account_info(),
                ctx.accounts.master_edition.to_account_info(),
                ctx.accounts.nft_mint.to_account_info()
            ],
            signer
        )?;

        // Update stake timestamp
        ctx.accounts.stake.timestamp = Clock::get()?.unix_timestamp.unsigned_abs();

        Ok(())
    }

    // Unstake NFT
    pub fn unstake(ctx: Context<UnstakeNFT>) -> Result<()> {
        // Calculate the reward based on the time staked
        let reward = Clock::get()?.unix_timestamp.unsigned_abs() - ctx.accounts.stake.timestamp;

        // Prepare authority for invoking thaw
        let authority_bump = *ctx.bumps.get("program_authority").unwrap();
        let authority_seeds = &["authority".as_bytes(), &[authority_bump]];
        let signer = &[&authority_seeds[..]];

        // Thaw the account
        let thaw_ix = thaw_delegated_account(
            ctx.accounts.metadata_program.key(), 
            ctx.accounts.program_authority.key(), 
            ctx.accounts.nft_token_account.key(), 
            ctx.accounts.master_edition.key(), 
            ctx.accounts.nft_mint.key()
        );

        // Invoke thaw with signature
        invoke_signed(
            &thaw_ix, 
            &[
                ctx.accounts.metadata_program.to_account_info(),
                ctx.accounts.program_authority.to_account_info(),
                ctx.accounts.nft_token_account.to_account_info(),
                ctx.accounts.master_edition.to_account_info(),
                ctx.accounts.nft_mint.to_account_info()
            ],
            signer
        )?;

        // Revoke staking approval
        revoke(ctx.accounts.revoke_ctx())?;

        // Mint reward token        
        let mint_bump = *ctx.bumps.get("mint_authority").unwrap();
        let mint_seeds = &["mint-authority".as_bytes(), &[mint_bump]];
        let signer = &[&mint_seeds[..]];

        let mint_to_ctx = ctx.accounts.mint_to_ctx().with_signer(signer);
        let result = mint_to(mint_to_ctx, reward);

        if result.is_err() {
            let error = result.err().unwrap();
            msg!("Mint {} reward token failed: {}", reward, error);
        }
        else{
            msg!("Mint {} reward token completed successfully.", reward);
        }

        // Close state account
        ctx.accounts.stake.close(ctx.accounts.user.to_account_info())?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateNFT<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        mint::decimals = 0,
        mint::authority = user,
        mint::freeze_authority = user
    )]
    pub nft_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = user,
        associated_token::mint = nft_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    /// CHECK: Safe metadata account
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), nft_mint.key().as_ref()],
        bump,
        seeds::program = metadata_program.key()
    )]
    pub metadata_account: AccountInfo<'info>,
    /// CHECK: Safe master edition account
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), nft_mint.key().as_ref(), b"edition"],
        bump,
        seeds::program = metadata_program.key()
    )]
    pub master_edition: AccountInfo<'info>,
    /// CHECK: Safe because verification through contraint
    #[account(
        constraint = metadata_program.key() == METADATA_PROGRAM_ID
    )]
    pub metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>
}

impl <'info> CreateNFT<'info> {
    /// Creates a CPI context for the `mint_to` instruction.
    pub fn mint_to_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        // Get the token program account info
        let cpi_program = self.token_program.to_account_info();
        
        // Prepare the CPI accounts structure
        let cpi_accounts = MintTo {
            mint: self.nft_mint.to_account_info(),          // Mint account of the NFT
            to: self.user_token_account.to_account_info(),  // Destination token account
            authority: self.user.to_account_info(),         // User's authority for minting
        };

        // Create a new CPI context with the prepared program and accounts
        CpiContext::new(cpi_program, cpi_accounts)
    }
}


#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(
        init,
        mint::authority = mint_authority,
        mint::decimals = 8, 
        seeds = ["token-mint".as_bytes()], 
        bump, 
        payer=payer)]
    pub token_mint: Account<'info, Mint>,
    #[account(seeds = ["mint-authority".as_bytes()], bump)]
    /// CHECK: using as signer
    pub mint_authority: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct StakeNFT<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub nft_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = user,
        seeds = [user.key().as_ref(), user_token_account.key().as_ref()],
        bump,
        space = 8 + 8
    )]
    pub stake: Account<'info, StakingData>,
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    /// CHECK: Manual validation
    #[account(owner=METADATA_PROGRAM_ID)]
    pub master_edition: UncheckedAccount<'info>,
    /// CHECK: Manual validation
    #[account(mut, seeds=["authority".as_bytes().as_ref()], bump)]
    pub program_authority: UncheckedAccount<'info>,
    /// CHECK: Safe because verification through contraint
    #[account(
        constraint = metadata_program.key() == METADATA_PROGRAM_ID
    )]
    pub metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct StakingData{
    pub timestamp: u64
}

impl <'info> StakeNFT<'info> {
    // Creates a CPI context for the `approve` instruction.
    pub fn approve_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Approve<'info>> {
        // Get the token program account info
        let cpi_program = self.token_program.to_account_info();
        
        // Prepare the CPI accounts structure
        let cpi_accounts = Approve { 
            to: self.user_token_account.to_account_info(),       // Destination token account
            delegate: self.program_authority.to_account_info(),  // Authority being delegated to
            authority: self.user.to_account_info()               // User's authority for approval
        };

        // Create a new CPI context with the prepared program and accounts
        CpiContext::new(cpi_program, cpi_accounts)
    }
}


#[derive(Accounts)]
pub struct UnstakeNFT<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [user.key().as_ref(), nft_token_account.key().as_ref()],
        bump
    )]
    pub stake: Account<'info, StakingData>,
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = user
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    /// CHECK: Manual validation
    #[account(owner=METADATA_PROGRAM_ID)]
    pub master_edition: UncheckedAccount<'info>,
    /// CHECK: Manual validation
    #[account(mut, seeds=["authority".as_bytes().as_ref()], bump)]
    pub program_authority: UncheckedAccount<'info>,
    /// CHECK: Safe because verification through contraint
    #[account(mut, seeds = ["token-mint".as_bytes()], bump)]
    pub token_mint: Account<'info, Mint>,
    #[account(mut, seeds = ["mint-authority".as_bytes()], bump)]
    /// CHECK: using as signer
    pub mint_authority: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_mint,
        associated_token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    /// CHECK: Safe because verification through contraint
    #[account(
        constraint = metadata_program.key() == METADATA_PROGRAM_ID
    )]
    pub metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl <'info> UnstakeNFT<'info> {
    // Creates a CPI context for the `revoke` instruction.
    pub fn revoke_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Revoke<'info>> {
        // Get the token program account info
        let cpi_program = self.token_program.to_account_info();
        
        // Prepare the CPI accounts structure for the `revoke` instruction
        let cpi_accounts = Revoke { 
            source: self.nft_token_account.to_account_info(),  // Source token account to revoke
            authority: self.user.to_account_info()              // User's authority for revoking
        };

        // Create a new CPI context with the prepared program and accounts
        CpiContext::new(cpi_program, cpi_accounts)
    }

    // Creates a CPI context for the `mint_to` instruction.
    pub fn mint_to_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        // Get the token program account info
        let cpi_program = self.token_program.to_account_info();
        
        // Prepare the CPI accounts structure for the `mint_to` instruction
        let cpi_accounts = MintTo {
            mint: self.token_mint.to_account_info(),             // Source mint for minting reward
            to: self.user_token_account.to_account_info(),      // Destination token account (user's reward)
            authority: self.mint_authority.to_account_info(),   // Mint authority for reward token
        };

        // Create a new CPI context with the prepared program and accounts
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
