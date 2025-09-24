import { ContractEventPayload, ethers } from 'ethers';
/**
 * @typedef {import('@metamask/providers').MetaMaskInpageProvider} MetaMaskInpageProvider
 */

/** @type {MetaMaskInpageProvider | undefined} */

const ethereum = window != undefined ? window.ethereum : undefined;
let active_pools_by_token = new Map();
let available_pools = new Map();
let available_tokens = new Map();
let available_dexes = new Map();

document.addEventListener('DOMContentLoaded',
  async function() {
    console.log("starting");
    await Start();
  }
)

let current_chain = 1;

async function Start() {

  await FetchChainDataAndUpdateStates(current_chain);

  let tokens_view = await BuildTokensView(json_data.tokens, json_data.dexes);

  if (ethereum != undefined) {
    current_chain = ethereum.chainId;
    ethereum.on('chainChanged', (chain_id) => { current_chain = chain_id; window.location.reload; });
  }

  document.body.appendChild(CreateHeader());
  document.body.appendChild(tokens_view);

}

async function FetchChainDataAndUpdateStates(chain_id) {

  let data = await fetch(`/${chain_id}`);
  let json_data = await data.json();
  console.log("json data received: \n {}", json_data);

  json_data.tokens.forEach(
    function(value) {
      if (available_tokens.has(chain_id)) {
        let tokens = available_tokens.get(chain_id);
        if (!tokens.has(value.address)) {
          dexes.set(value.address, value);
        }
      };
    }
  );

  json_data.dex.forEach(
    function(value) {
      if (available_dexes.has(chain_id)) {
        let dexes = available_dexes.get(chain_id);
        if (!dexes.has(value.address)) {
          dexes.set(value.address, value);
          }
        
      };
    }
  );

}

function UpdateChainPropertie(propertie,chain_id,new_data) {

  let map = new Map();
  if (!propertie.has(chain_id)) {
      propertie.set(chain_id,propertie);
    }else{
      map = propertie.get(chain_id);
    };

  data.forEach(function(value) {
    map.set(data.address,data);
  });
    
}

function DexesLoop(a, b, dexes) {
  const provider = new ethers.BrowserProvider(window.ethereum);
  let [a, b] = pair.split("-");
  console.log("a: {} \n b: {}", a, b);
  dexes.forEach(function(dex) {
    if (dex.version == 'v2') {
      console.log("calling v2 pool");
      console.log("dex {}", dex)
      console.log(window.ethereum
      );
      let contract = new ethers.Contract(dex.address, ["function getPair(address, address) view returns (address)"], provider);
      contract.getPair(a, b).then(res => UpdatePool(res));
    }
    else
      if (dex.version == 'v3') { }
      else
        if (dex.version == 'v4') { }

  });
}

function ActivateToken(updated_token) {
  //request from server,
  //look up for more in front,
  //send back to server

  let current_tokens = tokens.get(current_chain);

  console.log("current tokens loop {}", current_tokens);

  Array.from(current_tokens).forEach(function(value) {
    if (value.dataset.selected == "y") {
      //ord would require parsing to Address or something
      //simply testing both orders
      //usefull for v4 anyway

      console.log("value {}", value.dataset.address);
      console.log("updated {}", updated_token.dataset.address)

    }
  }
  );
}

async function _BuildTokensView(container) {
  let pools = document.createElement("div");
  pools.className = ('pools');
  for (let tkn_idx in tokens) {
    let token_view = document.createElement("button");
    token_view.className = "token_label_container";
    token_view.innerText = tokens[tkn_idx].symbol;
    token_view.dataset.address = tokens[tkn_idx].address;
    token_view.dataset.selected = "n";


    let onClickCallback = function() {
      if (token_view.dataset.selected == "n") {
        AddToken(token_view);
        console.log("enabling token");
        token_view.dataset.selected = "y";
      } else {
        RmvTokens(token_view);
        console.log("disabling token");
        token_view.dataset.selected = "n";
      }
    };

    token_view.addEventListener("click", onClickCallback);
    container.appendChild(token_view);
  }
}

async function BuildTokensView(tokens, dexes) {

  console.log("building tokens container");
  let tokens_view = document.getElementsByClassName("tokens_list_view")[0];
  if (tokens_view == undefined) {
    tokens_view = document.createElement("div");
    tokens_view.className = "tokens_list_view";
  }

  _BuildTokensView(tokens, dexes, tokens_view);
  return tokens_view;
}

function CreateHeader() {
  let header = document.createElement('div');
  header.className = 'header';
  header.appendChild(CreateHome());
  header.appendChild(CreateWalletBtn());

  return header;
}

function CreateHome() {
  let home = document.createElement('div');
  home.className = 'home';
  home.innerText = 'arbitro';
  return home;
}


function CreateWalletBtn() {
  let btn = document.createElement('button');
  btn.innerText = 'connect';
  btn.addEventListener('click', function() {
    console.log('pressed');

  });

  return btn;
}

