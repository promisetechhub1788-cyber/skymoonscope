/**
 * TypeScript type declarations for lib/stakingCalculator.js
 */

export interface StakingYieldInput {
  depositAmount?: number;
  lockDurationMonths?: number;
  compoundFrequency?: string;
  baseApyPercentage?: number;
  enableTierMultiplier?: boolean;
}

export interface MonthlyBreakdownRow {
  /** 1-based month index. */
  month: number;
  /** Total balance at this month (principal + accrued interest). */
  balance: number;
  /** Cumulative interest earned by this month. */
  yieldEarned: number;
}

export interface StakingYieldResult {
  depositAmount: number;
  lockDurationMonths: number;
  durationYears: number;
  baseApyPercentage: number;
  /** Duration tier multiplier applied to base APY (1.0 when disabled). */
  multiplier: number;
  /** baseApyPercentage × multiplier. */
  effectiveApyPercent: number;
  compoundFrequency: string;
  totalBalance: number;
  totalInterest: number;
  totalRoiPercent: number;
  estimatedDailyYield: number;
  estimatedMonthlyYield: number;
  breakdownByMonth: MonthlyBreakdownRow[];
}

export declare function calculateStakingYield(input?: StakingYieldInput): StakingYieldResult;
export declare function getDurationTierMultiplier(months: number): number;
export declare const COMPOUND_FREQUENCIES: Record<string, number>;
export declare const DURATION_TIER_MULTIPLIERS: Record<number, number>;
