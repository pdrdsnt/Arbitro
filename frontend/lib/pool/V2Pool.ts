export class V2Pool {
  factory: string;
  address: string;
  token0: string;
  token1: string;
  fee: number;
  constructor(
    address: string,
    token0: string,
    token1: string,
    fee: number,
    factory: string,
  ) {
    this.factory = factory;
    this.address = address;
    this.token0 = token0;
    this.token1 = token1;
    this.fee = fee;
  }
}
