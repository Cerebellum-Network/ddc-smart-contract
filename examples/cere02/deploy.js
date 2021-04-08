import { ApiPromise } from '@polkadot/api';
import { CodePromise } from '@polkadot/api-contracts';
// import { BlueprintPromise } from '@polkadot/api-contract'; // only if a codeHash is already existed

// Construct the API as per the API sections
// (as in all examples, this connects to a local chain)
const api = await ApiPromise.create();

// Construct our Code helper. The abi is an Abi object, an unparsed JSON string
// or the raw JSON data (after doing a JSON.parse). The wasm is either a hex
// string (0x prefixed), an Uint8Array or a Node.js Buffer object

let abi = '../target/ink/metadata.json';
let wasm = '../target/ink/ddc.wasm';

const code = new CodePromise(api, abi, wasm);

// Deploy the WASM, retrieve a Blueprint
let blueprint;

// createBlueprint is a normal submittable, so use signAndSend
// with an known Alice keypair (as per the API samples)
const unsub = await code
  .createBlueprint()
  .signAndSend(alicePair, (result) => {
    if (result.status.isInBlock || result.status.isFinalized) {
      // here we have an additional field in the result, containing the blueprint
      blueprint = result.blueprint;
      unsub();
    }
  })

// this step is redundant if we already get the blueprint from the above functions
// this step is only good if you've already have a codeHash on-chain, so you don't need to repeat the above steps
// const blueprint = new BlueprintPromise(api, abi, codeHash);

// Deploy a contract using the Blueprint
const endowment = 1230000000000n;

// NOTE The apps UI specifies these in Mgas
const gasLimit = 100000n * 1000000n;
const initValue = 123;

let contract;

// We pass the constructor (named `new` in the actual Abi),
// the endowment, gasLimit (weight) as well as any constructor params
// (in this case `new (initValue: i32)` is the constructor)
const unsub2 = await blueprint.tx
  .new(endowment, gasLimit, initValue)
  .signAndSend(alicePair, (result) => {
    if (result.status.isInBlock || result.status.isFinalized) {
      // here we have an additional field in the result, containing the contract
      contract = result.contract;
      unsub2();
    }
  });

// We pass the constructor (name, index or actual constructor from Abi),
// the endowment, gasLimit (weight) as well as any constructor params
// (in this case `new (initValue: i32)` is the constructor)

// const unsub3 = await blueprint
//   .createContract('new', endowment, gasLimit, initValue)
//   .signAndSend(alicePair, (result) => {
//     if (result.status.isInBlock || result.status.isFinalized) {
//         contract = result.contract;
//         unsub3();
//     }
//   });





