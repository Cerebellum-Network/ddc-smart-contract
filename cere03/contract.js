import { ApiPromise } from '@polkadot/api';
const ContractPromise = require('@polkadot/api-contract');

let api;

async function InitApi() {
    api = await ApiPromise.create();
}

InitApi();

// const api = await ApiPromise.create();


const contract_abi = '../target/ink/metadata.json';

const contract_address = '';

// Attach to an existing contract with a known ABI and address. As per the
// code and blueprint examples the abi is an Abi object, an unparsed JSON
// string or the raw JSON data (after doing a JSON.parse). The address is
// the actual on-chain address as ss58 or AccountId object.

const contract = new ContractPromise(api, contract_abi, contract_address);

// Read from the contract via an RPC call
const value = 0; // only useful on isPayable messages

// NOTE the apps UI specified these in mega units
const gasLimit = 3000n * 1000000n;

// Perform the actual read (no params at the end, for the `get` message)
// (We perform the send from an account, here using Alice's address)
//  the format is always of the form messageName(<account address to use>, <value>, <gasLimit>, <...additional params>)

// const { gasConsumed, result, outcome } = await contract.query.get(alicePair.address, { value, gasLimit });

async function pause_or_not() {
    let result = await contract.query.pause_or_not(alicePair.address, { value, gasLimit });
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
    console.log('gas consumed',gasConsumed.toHuman());
    // return result;
}

pause_or_not();

async function tier_deposit(tid) {
    let result = await contract.query.tier_deposit(alicePair, {value, gasLimit}, tid);
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

tier_deposit(3);


// the address we are going to query
const target = '5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY';
const from = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

// only 1 param needed, the actual address we are querying for (more
// params can follow at the end, separated by , if needed by the message)
// const callValue = await contract.query.balanceOf(from, { value: 0, gasLimit: -1 }, target);

async function balance_of(account) {
    let result = await contract.query.balance_of(from, {value, gasLimit}, account);
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

balance_of(target);

async function balance_of_contract() {
    let result = await contract.query.balance_of_contract(from, {value, gasLimit});
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

balance_of_contract();

async function token_symbol() {
    let result = await contract.query.token_symbol(from, {value, gasLimit});
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

token_symbol();

async function metrics_of(account) {
    let result = await contract.query.metrics_of(from, {value, gasLimit}, account);
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

metrics_of(target);

async function tier_id_of(account) {
    let result = await contract.query.tier_id_of(from, {value, gasLimit}, account);
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

tier_id_of(target);

async function tier_limit_of(account) {
    let result = await contract.query.tier_limit_of(from, {value, gasLimit}, account);
    if (result.isOk) {
        console.log('Success', result.toHuman());
      } else {
        console.error('Error', result.asErr);
    }
}

tier_limit_of(target);


async function transfer_ownership(acct1, acct2) {
    await contract.tx
    .transfer_ownership({ value, gasLimit }, acct1, acct2)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('finalized transfer ownership');
        }
    });
}

transfer_ownership(from, target);

async function change_tier_fee(tid, new_fee) {
    await contract.tx
    .change_tier_fee({ value, gasLimit }, tid, new_fee)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('tier fee changed');
        }
    });
}

change_tier_fee(3, 30000000000);

async function change_tier_limit(tid, new_limit) {
    await contract.tx
    .change_tier_fee({ value, gasLimit }, tid, new_limit)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('tier fee changed');
        }
    });
}

change_tier_limit(3, 3000);

async function create_payment(amount) {
    await contract.tx
    .create_payment({ value, gasLimit }, amount)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('finalized payment');
        }
    });
}

// not clear about the digits
create_payment(20000000000);

async function report_metrics(drec,drep,rrec,rrep) {
    await contract.tx
    .report_metrics({ value, gasLimit }, drec,drep,rrec,rrep)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('finalized report metrics');
        }
    });
}

report_metrics(1,2,3,4);

async function opt_out() {
    await contract.tx
    .opt_out({ value, gasLimit })
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('opt out');
        }
    });
}

opt_out();

async function revoke_membership(acct) {
    await contract.tx
    .revoke_membership({ value, gasLimit }, acct)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('membership revoked');
        }
    });
}

revoke_membership(from);

async function transfer_all_balance(acct) {
    await contract.tx
    .transfer_all_balance({ value, gasLimit }, acct)
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('transferred all the balance');
        }
    });
}

transfer_all_balance(target);

async function flip_contract_status() {
    await contract.tx
    .flip_contract_status({ value, gasLimit })
    .signAndSend(alicePair, (result) => {
        if (result.status.isInBlock) {
        console.log('in a block');
        } else if (result.status.isFinalized) {
        console.log('contract status flipped');
        }
    });
}

flip_contract_status();



// Instead of sending we use the `call` interface via `.query` that will return
// the gas consumed (the API aut-fill the max block tx weight when -1 is the gasLimit)
// const { gasConsumed, result } = await contract.query.inc(alicePair, { value, gasLimit: -1 }, incValue);

// console.log(`outcome: ${result.isOk ? 'Ok' : 'Error'}`);
// console.log(`gasConsumed ${gasConsumed.toString()}`);

