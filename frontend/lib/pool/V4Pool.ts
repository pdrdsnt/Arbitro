export class V4Pool {
  address: string;
  key: string;
  token0: string;
  token1: string;
  fee: number;
  tick_spacing: number;
  hooks: string;
  liquidity: bigint;

  constructor(
    address: string,
    key: string,
    token0: string,
    token1: string,
    fee: number,
    tick_spacing: number,
    hooks: string,
    liquidity: bigint,
  ) {
    this.key = key;
    this.address = address;
    this.token0 = token0;
    this.token1 = token1;
    this.fee = fee;
    this.tick_spacing = tick_spacing;
    this.hooks = hooks;
    this.liquidity = liquidity;
  }
}
