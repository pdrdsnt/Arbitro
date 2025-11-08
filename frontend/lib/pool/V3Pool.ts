export class V3Pool {
  address: string;
  token0: string;
  token1: string;
  tick_spacing: number;
  fee: number;
  factory: string;

  constructor(
    address: string,
    token0: string,
    token1: string,
    fee: number,
    tick_spacing: number,
    factory: string,
  ) {
    this.address = address;
    this.token0 = token0;
    this.token1 = token1;
    this.fee = fee;
    this.tick_spacing = tick_spacing;
    this.factory = factory;
  }
}
