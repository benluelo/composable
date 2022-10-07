import BigNumber from "bignumber.js";
import { ApiPromise } from "@polkadot/api";
import { fromChainIdUnit, fromPerbill } from "../unit";

type StakingPoolRewardRatePeriod = "PerSecond";

export class StakingPoolRewardRate {
  period: StakingPoolRewardRatePeriod;
  amount: BigNumber;

  static fromJSON(rewardRate: any): StakingPoolRewardRate {
    try {
      return new StakingPoolRewardRate(
        rewardRate.period,
        fromChainIdUnit(BigInt(rewardRate.amount))
      );
    } catch (err: any) {
      console.error("[StakingPoolRewardRate] ", err.message);
      throw new Error(err.message);
    }
  }

  constructor(period: StakingPoolRewardRatePeriod, amount: BigNumber) {
    this.period = period;
    this.amount = amount;
  }
}

export class StakingPoolReward {
  protected __assetId: BigNumber;
  protected __claimedRewards: BigNumber;
  protected __lastUpdatedTimestamp: number;
  protected __maxRewards: BigNumber;
  protected __rewardRate: StakingPoolRewardRate;
  protected __totalDilutionAdjustment: BigNumber;
  protected __totalRewards: BigNumber;

  static fromJSON(assetReward: any): StakingPoolReward {
    try {
      const assetId = new BigNumber(assetReward.assetId);
      const claimedRewards = fromChainIdUnit(
        BigInt(assetReward.claimedRewards)
      );
      const lastUpdatedTimestamp = new BigNumber(
        assetReward.lastUpdatedTimestamp
      ).toNumber();
      const maxRewards = fromChainIdUnit(BigInt(assetReward.maxRewards));
      const totalDilutionAdjustment = fromChainIdUnit(
        assetReward.totalDilutionAdjustment
      );
      const totalRewards = fromChainIdUnit(BigInt(assetReward.totalRewards));
      const rewardRate = StakingPoolRewardRate.fromJSON(assetReward.rewardRate);

      return new StakingPoolReward(
        assetId,
        claimedRewards,
        lastUpdatedTimestamp,
        maxRewards,
        rewardRate,
        totalDilutionAdjustment,
        totalRewards
      );
    } catch (err: any) {
      console.error("[StakingPoolReward] ", err.message);
      throw new Error(err.message);
    }
  }

  constructor(
    assetId: BigNumber,
    claimedRewards: BigNumber,
    lastUpdatedTimestamp: number,
    maxRewards: BigNumber,
    rewardRate: StakingPoolRewardRate,
    totalDilutionAdjustment: BigNumber,
    totalRewards: BigNumber
  ) {
    (this.__assetId = assetId), (this.__claimedRewards = claimedRewards);
    this.__lastUpdatedTimestamp = lastUpdatedTimestamp;
    this.__maxRewards = maxRewards;
    this.__rewardRate = rewardRate;
    this.__totalDilutionAdjustment = totalDilutionAdjustment;
    this.__totalRewards = totalRewards;
  }

  getAssetId(inBn: boolean = false): BigNumber | string {
    return inBn ? this.__assetId : this.__assetId.toString();
  }
}

export class StakingRewardPoolLockConfig {
  durationPresets: Map<number, BigNumber>;
  unlockPenalty: BigNumber;

  static fromJSON(lockConfig: any): StakingRewardPoolLockConfig {
    try {
      const unlockPenalty = fromPerbill(lockConfig.unlockPenalty);
      const durationPresets = new Map<number, BigNumber>();
      Object.keys(lockConfig.durationPresets).map((presetDuration) => {
        const presetDurationToNumber = new BigNumber(presetDuration).toNumber();
        const rewardMultiplier = fromPerbill(
          lockConfig.durationPresets[presetDuration]
        );
        durationPresets.set(presetDurationToNumber, rewardMultiplier);
      });

      return new StakingRewardPoolLockConfig(durationPresets, unlockPenalty);
    } catch (err: any) {
      console.error("[StakingRewardPoolLockConfig] ", err.message);
      throw new Error(err.message);
    }
  }

  constructor(
    durationPresets: Map<number, BigNumber>,
    unlockPenalty: BigNumber
  ) {
    this.durationPresets = durationPresets;
    this.unlockPenalty = unlockPenalty;
  }
}

export class StakingRewardPool {
  protected __api: ApiPromise;
  protected __assetId: BigNumber;
  protected __claimedShares: BigNumber;
  protected __endBlock: BigNumber;
  protected __totalShares: BigNumber;
  protected __shareAssetId: BigNumber;
  protected __financialNftAssetId: BigNumber;
  protected __owner: string;
  protected __lock: StakingRewardPoolLockConfig;
  protected __rewards: Map<string, StakingPoolReward>;

  static fromJSON(api: ApiPromise, stakePool: any): StakingRewardPool {
    try {
      const shareAssetId = new BigNumber(stakePool.shareAssetId);
      const financialNftAssetId = new BigNumber(stakePool.financialNftAssetId);
      const assetId = new BigNumber(stakePool.assetId);
      const claimedShares = new BigNumber(stakePool.claimedShares);
      const endBlock = new BigNumber(stakePool.endBlock);
      const owner = stakePool.owner;
      const lockConfig = StakingRewardPoolLockConfig.fromJSON(stakePool.lock);
      const totalShares = fromChainIdUnit(
        BigInt(stakePool.totalShares as string)
      );

      const rewards = new Map<string, StakingPoolReward>();
      Object.keys(stakePool.rewards).map((rewardAssetId) => {
        const stakingPoolReward = StakingPoolReward.fromJSON(
          stakePool.rewards[rewardAssetId]
        );
        rewards.set(
          stakingPoolReward.getAssetId(false) as string,
          stakingPoolReward
        );
      });

      return new StakingRewardPool(
        api,
        assetId,
        claimedShares,
        endBlock,
        totalShares,
        shareAssetId,
        financialNftAssetId,
        lockConfig,
        rewards,
        owner
      );
    } catch (err: any) {
      console.error("[StakingRewardPool] ", err.message);
      throw new Error(err.message);
    }
  }

  constructor(
    api: ApiPromise,
    assetId: BigNumber,
    claimedShares: BigNumber,
    endBlock: BigNumber,
    totalShares: BigNumber,
    shareAssetId: BigNumber,
    financialNftAssetId: BigNumber,
    stakePoolLockConfig: StakingRewardPoolLockConfig,
    rewards: Map<string, StakingPoolReward>,
    owner: string
  ) {
    this.__api = api;
    this.__assetId = assetId;
    this.__claimedShares = claimedShares;
    this.__endBlock = endBlock;
    this.__totalShares = totalShares;
    this.__shareAssetId = shareAssetId;
    this.__financialNftAssetId = financialNftAssetId;
    this.__lock = stakePoolLockConfig;
    this.__rewards = rewards;
    this.__owner = owner;
  }
}