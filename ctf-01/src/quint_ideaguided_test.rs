pub mod state_structs {
    use num_bigint::BigInt;
    use serde::Deserialize;
    use std::collections::HashMap;

    
    #[derive(Clone, Debug, Deserialize)]    
    pub struct WithdrawArgs {
        pub sender: String,
        pub lockup_ids: Vec<BigInt>
    }
    
    #[derive(Clone, Debug, Deserialize)] 
    pub struct DepositArgs {
        pub sender: String,
        pub amount: BigInt 
    }
    
    #[derive(Clone, Debug, Deserialize)]
    #[serde(tag = "tag", content = "value")]
    pub enum MsgArgs {
        NoArgs,
        DepositArgs(DepositArgs),
        WithdrawArgs(WithdrawArgs)
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StepInfo {
        pub action_taken: String,
        pub msg_args: MsgArgs,
        pub step_number: BigInt,
        pub action_error_description: String,
        pub action_successful: bool
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Lockup {
        pub owner: String,
        pub amount: BigInt,
        pub release_time: BigInt, 
    }

    #[derive(Clone, Debug, Deserialize)]    
    pub struct ContractState {
        pub free_id: BigInt,
        pub lockups: HashMap<BigInt, Lockup>,
        pub contract_balance: BigInt
    }

    #[derive(Clone, Debug, Deserialize)]    
    pub struct ChainState {
        pub time: BigInt
    }    

    #[derive(Clone, Debug, Deserialize)]    
    pub struct State {
        pub contract_state: ContractState,
        pub chain_state: ChainState,
        pub step_info: StepInfo
    }
}

#[cfg(test)]
pub mod tests {
    use itf::trace_from_str;
    use crate::{
        contract::{DENOM, LOCK_PERIOD, MINIMUM_DEPOSIT_AMOUNT},
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},        
        quint_ideaguided_test::state_structs::*,       
    };
    use cosmwasm_std::{coin, Addr, Uint128};
    use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
    use num_traits::ToPrimitive;
        
    
    pub const ADMIN: &str = "admin";
    pub const INIT_CONTRACT_COINS: Uint128 = Uint128::new(1_000_000);

    pub const INIT_USER_FUNDS: Uint128 = Uint128::new(1000000);
    // constants for users
    pub const USER_A: &str = "user_a";
    pub const USER_B: &str = "user_b";
    pub const USER_C: &str = "user_c";

    pub fn get_event_attribute(resp: &AppResponse, attribute_name: &str) -> String {
        let wasm = resp.events.iter().find(|x| x.ty == "wasm").unwrap();
        wasm.attributes
            .iter()
            .find(|x| x.key == attribute_name)
            .unwrap()
            .value
            .clone()     
    }
   
    pub fn mint_tokens(mut app: App, recipient: String, amount: Uint128) -> App {
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: recipient.to_owned(),
                amount: vec![coin(amount.u128(), DENOM)],
            },
        ))
        .unwrap();
        app
    }

    fn compare_state(test_state: &TestState, app: &App, state: &State) {  
        // compare contract balances
        let balance = app.wrap().query_balance(&test_state.contract_addr, DENOM).unwrap().amount;
        let trace_balance = state.contract_state.contract_balance.to_u128().unwrap();
        println!("Contract balance: {:?} vs {:?}", balance, Uint128::new(trace_balance));
        assert_eq!(balance, Uint128::new(trace_balance));


        // verify lockups
        // the contract's interface does not allow to do this properly:
        // we only check if lockups that exist in the trace have equivalents in the contract storage,
        // but not the other way around
        // in order to do this, one would need additional QueryMessages (e.g. GetNumberOfLockups),
        // but that would require changes to the CTF-01 interface
        for (id, trace_lockup) in &state.contract_state.lockups {
            let id = id.to_u64().unwrap();
            let msg = QueryMsg::GetLockup { id };
            let lockup: crate::state::Lockup = app
                .wrap()
                .query_wasm_smart(test_state.contract_addr.to_owned(), &msg)
                .unwrap();
            assert_eq!(
                lockup.amount,
                Uint128::new(trace_lockup.amount.to_u128().unwrap())
            );            
            assert_eq!(lockup.owner, trace_lockup.owner);
        }
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
            contract_addr: Addr::unchecked(""),
        };

        // load trace data
        let data = include_str!("../../generatedTraces/trace_ideaguided.itf.json");
        let trace: itf::Trace<State> = trace_from_str(data).unwrap();
        
        for s in trace.states {
            let step_info = &s.value.step_info;
            println!("Step number: {:?}", step_info.step_number);
            
            match step_info.action_taken.as_str() {    
                "init" => {
                    // this arm corresponds to the proper_instantiate function from the integration test
                    println!("Initializing contract.");

                    // init contract
                    
                    let msg = InstantiateMsg { count: 1i32 };
                    test_state.contract_addr = app
                        .instantiate_contract(
                            code_id,
                            Addr::unchecked(ADMIN),
                            &msg,
                            &[],
                            "test",
                            None,
                        )
                        .unwrap();


                    // mint funds to contract
                    app = mint_tokens(app, test_state.contract_addr.to_string(), INIT_CONTRACT_COINS);
                    // mint funds to users
                    app = mint_tokens(app, USER_A.to_string(), MINIMUM_DEPOSIT_AMOUNT * Uint128::new(10000));
                    app = mint_tokens(app, USER_B.to_string(), MINIMUM_DEPOSIT_AMOUNT * Uint128::new(10000));
                    app = mint_tokens(app, USER_C.to_string(), MINIMUM_DEPOSIT_AMOUNT * Uint128::new(10000));
                    println!("Contract initialized. Funds are {:?}", app.wrap().query_balance(USER_A.to_string(), DENOM).unwrap());
                } 
                "advance_time" => {
                    println!(
                        "clock is advancing for {} seconds (LOCK_PERIOD)",
                        LOCK_PERIOD
                    );

                    // fast forward LOCK_PERIOD seconds
                    app.update_block(|block| {
                        block.time = block.time.plus_seconds(LOCK_PERIOD);
                    });                    
                } 

                "deposit" => {
                    let deposit_args = match &step_info.msg_args {
                        MsgArgs::DepositArgs(deposit_args) => deposit_args,
                        _ => panic!("Invalid MsgArgs"),
                    };
                    println!("deposit_args: {:?}", deposit_args);
                    let msg = ExecuteMsg::Deposit {};
                    let sender = Addr::unchecked(&deposit_args.sender);
                    let res = app.execute_contract(
                        sender.clone(),
                        test_state.contract_addr.clone(),
                        &msg,
                        &[coin(deposit_args.amount.to_u128().unwrap(), DENOM)],
                    );

                    if step_info.action_successful {
                        println!("Deposit successful");
                        assert!(res.is_ok());                            
                    } else {
                        println!("Deposit failed");
                        assert!(res.is_err());                        
                    }
                    
                    
                }
                "withdraw" => {
                    if let MsgArgs::WithdrawArgs(withdraw_args) = &step_info.msg_args {
                        let ids: Vec<u64> = withdraw_args                            
                            .lockup_ids
                            .iter()
                            .map(|id| id.to_u64().unwrap())
                            .collect();

                        let sender = &withdraw_args.sender;
                        println!("user {} withdrawing from {:?}", sender, ids);

                        // send the withdrawal message
                        let msg = ExecuteMsg::Withdraw { ids };
                        let res = app.execute_contract(
                            Addr::unchecked(sender),
                            test_state.contract_addr.to_owned(),
                            &msg,
                            &[],
                        );
                        
                        if step_info.action_successful {
                            println!("Withdraw successful");
                            assert!(res.is_ok());                            
                        } else {
                            println!("Withdraw failed");
                            assert!(res.is_err());
                            
                        }
                        
                    } else 
                    {
                        println!("WITHDRAW: Wrong message arguments");
                        assert!(false);  
                    }
                    
                }
                _ => panic!("Invalid action taken"),
                
            }
            compare_state(&test_state, &app, &s.value);
            println!("-----------------------------------");
        }
        
    }
}
