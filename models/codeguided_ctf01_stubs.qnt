module codeguided_ctf01 {
  
  import basicSpells.* from "./lib_codeguided/basicSpells"
  import cw_types.* from "./lib_codeguided/cw_types"
  import cw_utils.* from "./lib_codeguided/cw_utils"
  import messaging.* from "./lib_codeguided/messaging"
  import bank from "./lib_codeguided/bank"

  
  var contract_state: ContractState
  var result: Result
  var bank: bank::Bank
  var time: int

  
  pure val CONTRACT_ADDRESS = "contract0"

  
  pure val ADDRESSES = Set("sender1", "sender2", "sender3", CONTRACT_ADDRESS)
  pure val DENOMS = Set("uawesome")
  // TODO: write the amount
  pure val MAX_AMOUNT = 200
  // This alias is added for name clarity: generated stubs give each
  // user (including the contract) the max amount of funds.
  // Here we want to differentiate between the two
  pure val INIT_CONTRACT_AMOUNT = MAX_AMOUNT


  type InstantiateMsg = { count: int }
  type Lockup = { id: int, owner: Addr, amount: int, release_timestamp: int }
  type ExecuteMsg =
    | ExecuteMsg_Deposit
    | ExecuteMsg_Withdraw({ ids: List[int] })
  pure def instantiate(state: ContractState, _env: Env, _info: MessageInfo, _msg: InstantiateMsg): (Result[Response, ContractError], ContractState) = {
    (Ok(Response_new.add_attribute("action", FromStr("instantiate"))), state)
  }

  pure def execute(state: ContractState, env: Env, info: MessageInfo, msg: ExecuteMsg): (Result[Response, ContractError], ContractState) = match msg {
    | ExecuteMsg_Deposit(__r) => deposit(state, env, info)
    | ExecuteMsg_Withdraw(__r) => withdraw(state, env, info, __r.ids)
  }

  pure def deposit(state: ContractState, env: Env, info: MessageInfo): (Result[Response, ContractError], ContractState) = 
      // TODO: specify the logic here
     (Ok(Response_new), state)
                            
  action deposit_action = {
    // TODO: Change next line according to fund expectations
    pure val max_funds = MAX_AMOUNT
    
    pure val message: ExecuteMsg = ExecuteMsg_Deposit
    execute_message(message, max_funds)
  }

  pure def withdraw(state: ContractState, env: Env, info: MessageInfo, ids: List[int]): (Result[Response, ContractError], ContractState) =
    // TODO: specify the logic here
      (Ok(Response_new), state)
                            
  action withdraw_action = all {
    contract_state.lockups.keys().size() > 0,
    // TODO: Change next line according to fund expectations
    pure val max_funds = MAX_AMOUNT
    
    nondet message_ids: List[int] = contract_state.lockups.keys().allListsUpTo(4).filter(l => l.length() > 0).oneOf()
    val message: ExecuteMsg = ExecuteMsg_Withdraw({ ids: message_ids })
    execute_message(message, max_funds)
  }

  // TODO: fill in the appropriate constant values
  pure val DENOM = "uawesome"
  pure val MINIMUM_DEPOSIT_AMOUNT = 100
  pure val LOCK_PERIOD = 60 * 60 * 24

  type ContractState = {
    last_id: int,
  lockups: int -> Lockup
  }

  pure val init_contract_state = {
        last_id: 1,
    lockups: Map()
  }

  action execute_step = all {
    any {
      deposit_action,
      withdraw_action
    },
    advance_time,
  }


  pure def reply(state: ContractState, _env: Env, _reply: Reply): (Result, ContractState) = (Ok(Response_new), state)



  pure val init_bank_state = ADDRESSES.mapBy(_ => DENOMS.mapBy(_ => MAX_AMOUNT))

  val env_val = { block: { time: time } }

  action init = {
    // TODO: Change next line according to fund expectations
    pure val max_funds = 0

    nondet sender = Set("admin").oneOf()
    nondet denom = DENOMS.oneOf()
    nondet amount = 0.to(max_funds).oneOf()
    val funds = [{ denom: denom, amount: amount }]
    val info = { sender: sender, funds: funds }

    pure val message: InstantiateMsg = { count: 0 }
    pure val r = instantiate(init_contract_state, { block: { time: 0 } }, info, message)

    all {
      contract_state' = r._2,
      bank' = init_bank_state,
      result' = r._1,
      time' = 0,
    }
  }


  action execute_message(message, max_funds) = {
    nondet sender = ADDRESSES.oneOf()
    nondet denom = DENOMS.oneOf()
    nondet amount = 0.to(max_funds).oneOf()
    val funds = [{ denom: denom, amount: amount }]
    val info = { sender: sender, funds: funds }

    val r = execute(contract_state, env_val, info, message)
    all {
      bank.get(sender).get(denom) >= amount,
      match r._1 {
        | Ok(_) => bank' = bank.setBy(sender, balances => balances.setBy(denom, balance => balance - amount))
                               .setBy(CONTRACT_ADDRESS, balances => balances.setBy(denom, balance => balance + amount))
        | Err(_) => bank' = bank
      },
      result' = r._1,
      contract_state' = r._2,
    }
  }

  action advance_time = time' = time + LOCK_PERIOD

  action step = {
    val message_getting = get_message(result)
    val new_result = message_getting._1
    val opt_message = message_getting._2
    match opt_message {
      | Some(submsg) => {
          val current_state = { bank: bank, result: new_result, contract_state: contract_state }
          val new_state = process_message(current_state, env_val, CONTRACT_ADDRESS, submsg, reply)
          all {
            bank' = new_state.bank,
            result' = new_state.result,
            contract_state' = new_state.contract_state,
            advance_time,
          }
      }
      | None => execute_step
    }
  }

  val contract_balance_ok = bank.get(CONTRACT_ADDRESS).get(DENOM) >= INIT_CONTRACT_AMOUNT // initial balance
}
