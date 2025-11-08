<script setup lang="ts">
import { MetaMaskInpageProvider } from "@metamask/providers";
import { onMounted, onUnmounted, Ref, ref, watch } from "vue";
import Tokens from "./Tokens.vue";
import { ChainData, Token } from "../lib/chain/ChainData";
import { AnyPool } from "../lib/pool/AnyPool.ts";
import { V2Factory, V2FactoryContract } from "../lib/factory/V2Factory";
import { V3Factory, V3FactoryContract } from "../lib/factory/V3Factory";
import { V4Factory, V4FactoryContract } from "../lib/factory/V4Factory";
import { Eip1193Provider, Result } from "ethers"; // If using ethers.js
import { OriginRequestChainData } from "../lib/OriginCalls.ts";
import { BrowserProvider } from "ethers";

declare global {
  interface Window {
    ethereum?: Eip1193Provider;
  }
}

console.log("mounting arbitro ");

let chain_id = ref(0);
let intervalId: string | number | NodeJS.Timeout | undefined;

let chains_data: Map<number, ChainData> = new Map();
let current_chain: Ref<undefined | ChainData> = ref(undefined);

const selected_tokens: Ref<Set<string>> = ref(new Set());
const active_pools_by_token: Ref<Map<string, Map<string, AnyPool>>> = ref(
  new Map(),
);

const insertToken = (token: Token) => {
  if (current_chain.value === undefined) {
    current_chain.value = chains_data.get(chain_id.value);
  }
  if (current_chain.value === undefined) {
    return;
  }

  console.log("current chain id from ref to data: ", current_chain.value?.id);
  console.log("current chain id from vue component: ", chain_id.value);
  const pairs: [string, string][] = [];

  console.log("tokens selected: ", token);

  if (selected_tokens.value.has(token.address)) {
    console.log("selected tokens is on list, removing ", token.symbol);
    selected_tokens.value.delete(token.address);
    active_pools_by_token.value.delete(token.address);
  } else {
    console.log("selected tokens is not on list, adding ", token.symbol);
    for (const tkn of selected_tokens.value) {
      const pair: [string, string] = [tkn, token.address];
      pairs.push(pair);
    }

    let search_pool_calls: Promise<AnyPool | { error: string }>[] = [];

    const _eth = window.ethereum;
    if (!_eth) {
      return;
    }

    if (current_chain.value) {
      for (const dex of current_chain.value.factories) {
        let contract = dex.contract;
        console.log("dex: ", dex);

        const eth = new BrowserProvider(_eth);
        if (!eth) return;
        if (dex instanceof V2Factory) {
          contract = new V2FactoryContract(
            dex,
            _eth,
            current_chain.value.pools,
          );
        } else if (dex instanceof V3Factory) {
          contract = new V3FactoryContract(
            dex,
            _eth,
            current_chain.value.pools,
          );
        } else if (dex instanceof V4Factory) {
          contract = new V4FactoryContract(
            dex,
            _eth,
            current_chain.value.pools,
          );
        }

        if (contract) {
          for (const pair of pairs) {
            const calls = contract.searchPair(pair[0], pair[1]);
            search_pool_calls.concat(...calls);
          }
        }

        for (const call of search_pool_calls) {
          call
            .then((result) => {
              const pool = result as AnyPool;
              if (pool) {
                const a = pool.token0;
                const b = pool.token1;
                const a_map = active_pools_by_token.value.get(a);
                const b_map = active_pools_by_token.value.get(b);

                console.log("pool: ", pool);

                if (a_map) {
                  a_map.set(pool.address, pool);
                } else {
                  let new_pools_map = new Map();
                  new_pools_map.set(pool.address, pool);
                  active_pools_by_token.value.set(a, new_pools_map);
                }
                if (b_map) {
                  b_map.set(pool.address, pool);
                } else {
                  let new_pools_map = new Map();
                  new_pools_map.set(pool.address, pool);
                  active_pools_by_token.value.set(b, new_pools_map);
                }
              }
            })
            .catch((err) => {
              console.log("err: ", err);
            });
        }
      }
    }
    selected_tokens.value.add(token.address);
  }
};

watch(chain_id, (newVal) => {
  let chain = chains_data.get(newVal);

  console.log("chain id changed", newVal);

  if (!chain) {
    console.log("creating chain data for: ", newVal);
    OriginRequestChainData(newVal).then((chain_data) => {
      if (chain_data) {
        chains_data.set(newVal, chain_data);
        console.log("current chain data: ", chain_data);
        current_chain.value = chain_data;
        console.log("current chain data: ", current_chain.value);
      }
    });
  } else {
    current_chain.value = chain;
    console.log("current chain data: ", current_chain.value);
  }
});

onMounted(() => {
  const syncChain = () => {
    const eth = window.ethereum;
    if (!eth) return;
    eth
      .request({ method: "eth_chainId" })
      .then((_id) => {
        console.log("id: ", _id);

        let id = parseInt(_id as string, 16);
        if (chain_id.value !== id) {
          console.log("chain changed", id);
          chain_id.value = id;
        }
      })
      .catch((error) => {
        console.log("error requesting chain", error);
      });
  };

  intervalId = setInterval(syncChain, 1000);
});

onUnmounted(() => {
  clearInterval(intervalId);
});

function test() {
  console.log("token clicked");
}
</script>
<template>
  <div class="hhhh">
    <h2>{{ chain_id }}</h2>
    <div v-if="current_chain">
      <div class="tlist">
        <div v-for="token in current_chain.tokens.values()">
          <div v-if="!selected_tokens.has(token.address)">
            <button class="a" v-on:click="insertToken(token)">
              {{ token.symbol }}
            </button>
          </div>
          <div v-else>
            <button class="b" v-on:click="insertToken(token)">
              {{ token.symbol }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.hhhh {
  display: flexbox;
  background-color: yellow;
  margin: 6px;
}
.tlist {
  display: flex;
  overflow-x: scroll;
  background-color: lightgoldenrodyellow;
}
.a {
  color: grey;
  background-color: goldenrod;
}
.b {
  color: blue;
  background-color: red;
}
</style>
