import { MetaMaskInpageProvider } from "@metamask/providers";
import { V2Factory, V2FactoryContract } from "./V2Factory";
import { V3Factory, V3FactoryContract } from "./V3Factory";
import { V4Factory, V4FactoryContract } from "./V4Factory";
import { getAddress, isAddress } from "ethers";

export type AnyFactory = V2Factory | V3Factory | V4Factory;

export type AnyFactoryContract =
  | V2FactoryContract
  | V3FactoryContract
  | V4FactoryContract;

export function FactoryBuilder(json_data): AnyFactory | null {
  let version = json_data.version;
  let address: string = json_data.address;
  if (isAddress(address)) {
    json_data.address = getAddress(address);

    if (version) {
      if (json_data.version == "v2") {
        return new V2Factory(json_data);
      } else if (json_data.version == "v3") {
        return new V3Factory(json_data);
      } else if (json_data.version == "v4") {
        return new V4Factory(json_data);
      }
    }
  }
  return null;
}
