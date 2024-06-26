pub mod state_structs {
    use itf::de::{self, As};
    use num_bigint::BigInt;
    use serde::Deserialize;
    use std::collections::HashMap;
    #[derive(Clone, Debug, Deserialize)]
    pub struct Lockup {
        pub id: BigInt,
        pub owner: String,
        pub amount: BigInt,
        pub release_timestamp: BigInt,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct InstantiateMsg {
        pub count: BigInt,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct ContractState {
        pub last_id: BigInt,
        pub lockups: HashMap<BigInt, Lockup>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct NondetPicks {
        #[serde(with = "As::<de::Option::<_>>")]
        pub sender: Option<String>,

        #[serde(with = "As::<de::Option::<_>>")]
        pub denom: Option<String>,

        #[serde(with = "As::<de::Option::<_>>")]
        pub amount: Option<BigInt>,

        #[serde(with = "As::<de::Option::<_>>")]
        pub message_ids: Option<Vec<BigInt>>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Message {}

    #[derive(Clone, Debug, Deserialize)]
    pub struct Response {
        pub messages: Vec<Message>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct State {
        pub contract_state: ContractState,
        pub bank: HashMap<String, HashMap<String, BigInt>>,
        #[serde(with = "As::<de::Result::<_, _>>")]
        pub result: Result<Response, String>,
        pub action_taken: String,
        pub nondet_picks: NondetPicks,
        pub time: BigInt,
    }
}
#[cfg(test)]
pub mod tests {
    use crate::{
        contract::LOCK_PERIOD,
        quint_codeguided_test::state_structs::*,
        msg::{ExecuteMsg, InstantiateMsg},
    };
    use cosmwasm_std::{coin, Addr, Uint128};
    use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
    use itf::trace_from_str;
    use num_bigint::BigInt;
    use num_traits::{ToPrimitive, Zero};

    pub const DENOM: &str = "uawesome";
    pub const TICK: u64 = LOCK_PERIOD;

    pub fn mint_tokens(mut app: App, recipient: String, denom: String, amount: Uint128) -> App {
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: recipient.to_owned(),
                amount: vec![coin(amount.u128(), denom)],
            },
        ))
        .unwrap();
        app
    }

    fn compare_state(test_state: &TestState, app: &App, state: &State) {
        // compare contract balances
        let balance = app
            .wrap()
            .query_balance(&test_state.contract_addr, DENOM)
            .unwrap()
            .amount;
        let trace_balance = state
            .bank
            .get(&test_state.contract_addr.to_string())
            .and_then(|x| x.get(DENOM))
            .and_then(|x| x.to_u128())
            .unwrap_or(0);
        println!(
            "Contract balance ({:?}) for {DENOM}: {:?} vs {:?}",
            test_state.contract_addr,
            balance,
            Uint128::new(trace_balance)
        );
        assert_eq!(balance, Uint128::new(trace_balance));

        // TODO: Query the contract and compare the state as you wish
    }

    fn compare_result(
        trace_result: Result<Response, String>,
        app_result: Result<AppResponse, anyhow::Error>,
    ) {
        if trace_result.is_ok() {
            assert!(
                app_result.is_ok(),
                "Action unexpectedly failed, error: {:?}",
                app_result.err()
            );
            println!("Action successful as expected");
        } else {
            assert!(
                app_result.is_err(),
                "Expected action to fail with error: {:?}",
                trace_result.err()
            );
            println!("Action failed as expected");
        }
    }

    fn funds_from_trace(amount: Option<BigInt>, denom: Option<String>) -> Vec<cosmwasm_std::Coin> {
        if amount.is_none() || denom.is_none() || amount == Some(Zero::zero()) {
            return vec![];
        }

        vec![coin(
            amount.as_ref().unwrap().to_u128().unwrap(),
            denom.unwrap(),
        )]
    }

    // Testing is stateful.
    struct TestState {
        // we will only know the contract address once we have processed an `instantiate` step
        pub contract_addr: Addr,
    }

    #[test]
    fn model_test() {
        let mut app = App::default();
        let code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        let code_id = app.store_code(Box::new(code));

        // create test state
        let mut test_state = TestState {
            contract_addr: Addr::unchecked("contract0"),
        };

        // load trace data
        let data = include_str!("../../generatedTraces/trace_codeguided.itf.json");
        
        let trace: itf::Trace<State> = trace_from_str(data).unwrap();

        for s in trace.states {
            let last_result = s.value.result.clone();
            if last_result.is_ok() && !last_result.unwrap().messages.is_empty() {
                println!("Processing messages, skipping");
                continue;
            }

            let action_taken = &s.value.action_taken;
            let nondet_picks = &s.value.nondet_picks;
            let amount = nondet_picks.amount.clone();
            let denom = nondet_picks.denom.clone();
            let sender = nondet_picks.sender.clone();

            println!("Step number: {:?}", s.meta.index);

            match action_taken.as_str() {
                "deposit_action" => {
                    let sender = Addr::unchecked(sender.unwrap());
                    let funds = funds_from_trace(amount, denom);

                    let msg = ExecuteMsg::Deposit {};
                    println!("Message: {:?}", msg);
                    println!("Sender: {:?}", sender);
                    println!("Funds: {:?}", funds);

                    let res = app.execute_contract(
                        sender,
                        test_state.contract_addr.clone(),
                        &msg,
                        &funds,
                    );

                    compare_result(s.value.result.clone(), res)
                }

                "withdraw_action" => {
                    let sender = Addr::unchecked(sender.unwrap());
                    let funds = funds_from_trace(amount, denom);

                    let message_ids = nondet_picks
                        .message_ids
                        .clone()
                        .unwrap()
                        .iter()
                        .map(|x| x.to_u64().unwrap())
                        .collect();
                    let msg = ExecuteMsg::Withdraw { ids: message_ids };
                    println!("Message: {:?}", msg);
                    println!("Sender: {:?}", sender);
                    println!("Funds: {:?}", funds);

                    let res = app.execute_contract(
                        sender,
                        test_state.contract_addr.clone(),
                        &msg,
                        &funds,
                    );

                    compare_result(s.value.result.clone(), res)
                }

                "q::init" => {
                    println!("Initializing contract.");

                    let sender = Addr::unchecked(sender.unwrap());
                    let funds = funds_from_trace(amount, denom);

                    let msg = InstantiateMsg { count: 0 };
                    println!("Message: {:?}", msg);
                    println!("Sender: {:?}", sender);
                    println!("Funds: {:?}", funds);

                    test_state.contract_addr = app
                        .instantiate_contract(code_id, sender, &msg, &funds, "test", None)
                        .unwrap();

                    for (addr, coins) in s.value.bank.clone().iter() {
                        for (denom, amount) in coins.iter() {
                            app = mint_tokens(
                                app,
                                addr.clone(),
                                denom.to_string(),
                                Uint128::new(amount.to_u128().unwrap()),
                            );
                        }
                    }
                }

                _ => panic!("Invalid action taken"),
            }
            compare_state(&test_state, &app, &(s.value.clone()));
            println!("clock is advancing for {} seconds", TICK);
            app.update_block(|block| {
                block.time = block.time.plus_seconds(TICK);
            });
            println!("-----------------------------------");
        }
    }
}
