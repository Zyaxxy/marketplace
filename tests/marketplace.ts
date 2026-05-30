import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { Marketplace } from "../target/types/marketplace";
import { assert } from "chai";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { MPL_CORE_PROGRAM_ID, create, mplCore} from "@metaplex-foundation/mpl-core";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { generateSigner, keypairIdentity } from "@metaplex-foundation/umi";
import { fromWeb3JsKeypair } from "@metaplex-foundation/umi-web3js-adapters";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import BN from "bn.js";
import { PublicKey } from "@metaplex-foundation/umi";

const FEE_BPS = 500;
const PRICE_LAMPORTS = new BN(1_000_000);
const TOKEN_PRICE = new BN(2_000_000);
const REWARD_BPS = 100;



describe("marketplace", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.marketplace as Program<Marketplace>;
  const connection = provider.connection;
  const wallet = provider.wallet as NodeWallet;
  const payer = wallet.payer;
  const maker = web3.Keypair.generate();
  const taker = web3.Keypair.generate();
  const commitment = "confirmed";

  const marketplaceName = `mkt-${Date.now().toString().slice(-8)}`;
  let asset: web3.PublicKey;
  let collection: PublicKey;

  const [marketplace] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("marketplace"), Buffer.from(marketplaceName)],
    program.programId
  );
  const [treasury] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), marketplace.toBuffer()],
    program.programId
  );
  const [rewardMint] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), marketplace.toBuffer()],
    program.programId
  );

  const getListingPda = (assetKey: web3.PublicKey) =>
    web3.PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), assetKey.toBuffer()],
      program.programId
    )[0];

  const getRewardAta = () =>
    getAssociatedTokenAddressSync(
      rewardMint,
      taker.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

  const toBigInt = (value: BN): bigint => BigInt(value.toString());

  const getTokenBalance = async (account: web3.PublicKey): Promise<BN> => {
    const balance = await connection.getTokenAccountBalance(account);
    return new BN(balance.value.amount);
  };

  const confirmTx = async (signature: string) => {
    const latestBlockHash = await connection.getLatestBlockhash();
    await connection.confirmTransaction(
      {
        signature,
        ...latestBlockHash,
      },
      commitment
    );
  };

  const confirmTxs = async (signatures: string[]) => {
    await Promise.all(signatures.map(confirmTx));
  };

  before(async () => {
    const signatures = await Promise.all(
      [maker, taker].map((keypair) =>
        connection.requestAirdrop(
          keypair.publicKey,
          2 * web3.LAMPORTS_PER_SOL
        )
      )
    );
    await confirmTxs(signatures);
    
    const umi = createUmi(connection).use(mplCore());
    umi.use(keypairIdentity(fromWeb3JsKeypair(maker)));

    const assetSigner = generateSigner(umi);
    asset = new web3.PublicKey(assetSigner.publicKey);
    const assetUri = "https://example.com/asset.json";

    await create(umi, {
      asset: assetSigner,
      name: "Marketplace asset",
      uri: assetUri,
      owner: umi.identity.publicKey,
    }).sendAndConfirm(umi);

    console.log("Created asset:", asset.toBase58());
    
  });

  it("initializes the marketplace", async () => {
    const tx = await program.methods
      .initialize(marketplaceName, FEE_BPS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        marketplace,
        treasury,
        rewardMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize tx:", tx);

    const marketplaceAccount = await program.account.marketplace.fetch(
      marketplace
    );
    assert.equal(marketplaceAccount.name, marketplaceName);
    assert.equal(marketplaceAccount.fee, FEE_BPS);
    assert.equal(
      marketplaceAccount.admin.toBase58(),
      provider.wallet.publicKey.toBase58()
    );
  });

  it("lists and delists an asset", async () => {
    const listing = getListingPda(asset);

    const listTx = await program.methods
      .list(PRICE_LAMPORTS)
      .accountsPartial({
        maker: maker.publicKey,
        asset,
        collection: null,
        paymentMint: NATIVE_MINT,
        listing,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    console.log("List tx:", listTx);

    const listingAccount = await program.account.listing.fetch(listing);
    assert.equal(listingAccount.price.toString(), PRICE_LAMPORTS.toString());
    assert.equal(listingAccount.paymentMint.toBase58(), NATIVE_MINT.toBase58());

    const delistTx = await program.methods
      .delist()
      .accountsPartial({
        maker: maker.publicKey,
        listing,
        asset,
        collection: null,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    console.log("Delist tx:", delistTx);

    const listingInfo = await connection.getAccountInfo(listing);
    assert.isNull(listingInfo, "Listing account should be closed after delist");
  });

  it("buys with SOL", async () => {
    const listing = getListingPda(asset);

    const listTx = await program.methods
      .list(PRICE_LAMPORTS)
      .accountsPartial({
        maker: maker.publicKey,
        asset,
        collection: null,
        paymentMint: NATIVE_MINT,
        listing,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    console.log("List for buy (SOL) tx:", listTx);

    const rewardAta = getRewardAta();
    const rewardBefore = await getTokenBalance(rewardAta).catch(
      () => new BN(0)
    );
    const treasuryBefore = new BN(
      (await connection.getBalance(treasury)).toString()
    );

    const buyTx = await program.methods
      .buy()
      .accountsPartial({
        taker: taker.publicKey,
        maker: maker.publicKey,
        asset,
        collection: null,
        marketplace,
        listing,
        treasury,
        rewardMint,
        takerRewardAta: rewardAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    console.log("Buy (SOL) tx:", buyTx);

    const rewardAfter = await getTokenBalance(rewardAta);
    const expectedReward = PRICE_LAMPORTS.muln(REWARD_BPS).divn(10000);
    assert.isTrue(
      rewardAfter.sub(rewardBefore).eq(expectedReward),
      "Rewards should be minted at 1% of price"
    );

    const treasuryAfter = new BN(
      (await connection.getBalance(treasury)).toString()
    );
    const expectedFee = PRICE_LAMPORTS.muln(FEE_BPS).divn(10000);
    assert.isTrue(
      treasuryAfter.sub(treasuryBefore).eq(expectedFee),
      "Treasury should receive the SOL fee"
    );

    const listingInfo = await connection.getAccountInfo(listing);
    assert.isNull(listingInfo, "Listing account should be closed after buy");
  });

  it("buys with tokens", async () => {
    const makerUmi = fromWeb3JsKeypair(maker);
    const umi = createUmi(connection).use(mplCore());
    umi.use(keypairIdentity(makerUmi));

    const tokenAssetSigner = generateSigner(umi);
    asset = new web3.PublicKey(tokenAssetSigner.publicKey);

    await create(umi, {
      asset: tokenAssetSigner,
      name: "Marketplace token asset",
      uri: "https://example.com/token-asset.json",
      owner: makerUmi.publicKey,
    }).sendAndConfirm(umi);

    const paymentMint = await createMint(
      connection,
      payer,
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      6
    );

    const takerPaymentAta = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        paymentMint,
        taker.publicKey
      )
    ).address;

    const makerPaymentAta = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        paymentMint,
        maker.publicKey
      )
    ).address;

    await mintTo(
      connection,
      payer,
      paymentMint,
      takerPaymentAta,
      provider.wallet.publicKey,
      toBigInt(TOKEN_PRICE)
    );

    const listing = getListingPda(asset);
    const listTx = await program.methods
      .list(TOKEN_PRICE)
      .accountsPartial({
        maker: maker.publicKey,
        asset,
        collection: null,
        paymentMint,
        listing,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    console.log("List for buy (token) tx:", listTx);

    const [treasuryPaymentAccount] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("treasury"),
        marketplace.toBuffer(),
        paymentMint.toBuffer(),
      ],
      program.programId
    );

    const rewardAta = getRewardAta();
    const rewardBefore = await getTokenBalance(rewardAta).catch(
      () => new BN(0)
    );
    const takerBalanceBefore = await getTokenBalance(takerPaymentAta);

    const buyTx = await program.methods
      .buyWithToken()
      .accountsPartial({
        taker: taker.publicKey,
        maker: maker.publicKey,
        asset,
        collection:null,
        marketplace,
        listing,
        paymentMint,
        takerPaymentAta,
        makerPaymentAta,
        treasuryPaymentAccount,
        rewardMint,
        takerRewardAta: rewardAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    console.log("Buy (token) tx:", buyTx);

    const rewardAfter = await getTokenBalance(rewardAta);
    const expectedReward = TOKEN_PRICE.muln(REWARD_BPS).divn(10000);
    assert.isTrue(
      rewardAfter.sub(rewardBefore).eq(expectedReward),
      "Rewards should be minted at 1% of price"
    );

    const treasuryBalance = await getTokenBalance(treasuryPaymentAccount);
    const expectedFee = TOKEN_PRICE.muln(FEE_BPS).divn(10000);
    assert.isTrue(
      treasuryBalance.eq(expectedFee),
      "Treasury token account should receive the fee"
    );

    const takerBalanceAfter = await getTokenBalance(takerPaymentAta);
    assert.isTrue(
      takerBalanceBefore.sub(takerBalanceAfter).eq(TOKEN_PRICE),
      "Taker should pay the full token price"
    );

    const makerBalance = await getTokenBalance(makerPaymentAta);
    const expectedMakerAmount = TOKEN_PRICE.sub(expectedFee);
    assert.isTrue(
      makerBalance.eq(expectedMakerAmount),
      "Maker should receive price minus fee"
    );

    const listingInfo = await connection.getAccountInfo(listing);
    assert.isNull(listingInfo, "Listing account should be closed after buy");
  });
});
