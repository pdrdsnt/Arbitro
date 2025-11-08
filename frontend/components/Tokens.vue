<script setup lang="ts">
import { ref, Ref } from "vue";
import { Token } from "../lib/chain/ChainData.ts";

const props = defineProps({ available_tokens: Array<Token> });
const emit = defineEmits(["addToken", "rmvToken"]);
const tokens_view = ref<Token[]>([]);
const selected = ref(new Set());

const selectToken = (token: string) => {
  if (selected.value.has(token)) {
    selected.value.delete(token);
    emit("addToken", [token]);
  } else {
    selected.value.add(token);
    emit("rmvToken", [token]);
  }
};
</script>
<template>
  <div>
    <div v-for="token in available_tokens" :key="token.address">
      <button v-on:click="selectToken(token.address)">
        {{ token.symbol }}
      </button>
    </div>
  </div>
</template>
