import { ChainData } from "./chain/ChainData";

export async function OriginRequestChainData(
  chain_id: number,
): Promise<ChainData> {
  let data = await fetch(`/${chain_id}`);
  let json_data = await data.json();
  console.log("chain data requested from origin: ", json_data);
  return new ChainData(json_data, chain_id);
}

export async function OriginRequestAvailableChains(): Promise<number[]> {
  let data = await fetch(`/chains`);
  let json_data = await data.json();
  return json_data as number[];
}
