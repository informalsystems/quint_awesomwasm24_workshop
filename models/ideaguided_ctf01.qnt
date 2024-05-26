// -*- mode: Bluespec; -*-


// The goal of this model is to model the behavior of the contract "as it should be".
// In the extreme, the model is built from the English-language specification, not being aware of the implementation.
// This is, of course, not realistic: we need to at least know the interface of the contract.
// (And later on, we will need to know more details to write an adapter, eg constants.)
module ideaguided_ctf01 {

    import types.* from "lib_ideaguided/types"
    import constants.* from "lib_ideaguided/constants"

    // Quint standard libraries
    import basicSpells.* from "spells/basicSpells"
    // import commonSpells.* from "../spells/commonSpells"
    import rareSpells.* from "spells/rareSpells"
    
    // VARIABLES
    // state of the contract that is being modeled
    var contract_state: ContractState
    // state of the chain: this could in general include many aspects of the environment 
    // external to the contract. The only thing that matters to us is the time.
    var chain_state: ChainState  
    // helper state variable, used to inform the adapter about the details of the 
    // action taken and its result
    var step_info: StepInfo


    // -------------------------
    // --- The State Machine ---
    // -------------------------
    // The state machine in Quint is defined by actions: functions that update state variables.


    // INITIAL STATE
    // The initial state: a special action which defines state variables at the beginning of the model execution
    action init = all {
        
        step_info' = {
            actionTaken: "init",
            msgArgs: NoArgs,
            actionSuccessful: true,
            actionErrorDescription: "",
            stepNumber: 0
        },
        chain_state' = {
            time: INIT_TIME
        },
        contract_state' = {
            free_id: INIT_FREE_ID,
            contract_balance: INIT_CONTRACT_BALANCE,
            lockups: Map()
        }
    }

    // MODEL EVOLUTION
    // Definition of the transition function
    action step = all {   

        any{
            nondet sender = oneOf(SENDERS)
            nondet amount = oneOf(AMOUNTS)
            deposit(sender, amount),

            nondet sender = oneOf(SENDERS)
            nondet num_lockups_to_withdraw = oneOf(1.to(MAX_LOCKUPS_TO_WITHDRAW))
            val available_lockup_ids = contract_state.lockups.keys()
            
            // this is a modeling choice: we will never see in our model executions some 
            // potentially interesting error scenarios. It would be a legitimate choice to 
            // allow all possible ids to occur here, and then model in the `step_info` that
            // an error should happen.
            val filtered_lockup_ids = available_lockup_ids.filter(
                id =>                 
                and(
                    contract_state.lockups.get(id).owner == sender,
                    contract_state.lockups.get(id).release_time <= chain_state.time                    
                )
            )
            all{ 
                require(size(filtered_lockup_ids) > 0),                
                // THIS IS A more elegant way, but one cannot use `quint verify` on it, only `quint run`
                // nondet selected_indices: List[int] = oneOf(
                //     filtered_lockup_ids
                //         // ATM, this operator works only with `quint run ...`, but not with `quint verify ...`
                //         .allListsUpTo(4)
                //         .filter(l => l.length() > 0)                    
                //     )


                // This is a less elegant way of generating lists, in order to enable usage 
                // of `quint verify` until the support for `allListsUpTo` is added to Apalache
                // By doing so, we limit our lists to the size of (at most) 3.
                val possibilities = filtered_lockup_ids
                                        .map(i => Some(i))
                                        .union(Set(None))
                nondet v1 = possibilities.oneOf()
                nondet v2 = possibilities.oneOf()
                nondet v3 = possibilities.oneOf()
                val selected_indices: List[int] = 
                    [v1, v2, v3]
                        .foldl(
                            [], 
                            (acc, v) => match v {
                                | Some(i) => acc.append(i)
                                | None => acc
                                }
                            )

                            
                withdraw(sender, selected_indices)
            },

            advance_time_lock_period
        }
    }  

    // ACTIONS
    action advance_time_lock_period = all { 
        chain_state' = {
            time: chain_state.time + LOCK_PERIOD
        },
        step_info' = {
            actionTaken: "advance_time",
            msgArgs: NoArgs,
            actionSuccessful: true,
            actionErrorDescription: "",
            stepNumber: step_info.stepNumber + 1
        },
        contract_state' = contract_state
    }

    action deposit(sender: Addr, amount: int): bool = {
        all {
            // this is a choice: we forbid depositing non-positive amounts
            // (we could have allow it - this would allow us testing for more errors)
            // In the end, it is a modeling decision
            require(amount > 0),
            contract_state' = deposit_pure(contract_state, sender, amount, chain_state.time),
            chain_state' = chain_state,
            // at the moment, we do not model any failure at depositing
            step_info' = {
                actionTaken: "deposit",
                msgArgs: DepositArgs({sender: sender, amount: amount}),
                actionSuccessful: true,
                actionErrorDescription: "",
                stepNumber: step_info.stepNumber + 1
            }
        }
    }        
    
    action withdraw(sender: Addr, ids: List[LockupId]): bool = {
        all {
        // since the ChainState only models time, we do not update it here:
        // one block time is negligible compared to the unlock period (1 day).
        // Thus, we only model time advancing in the action `advance_time_lock_period`.
        chain_state' = chain_state,

        // update the info for the adapter 
        step_info' = {
            actionTaken: "withdraw",
            msgArgs: WithdrawArgs({sender: sender, lockup_ids: ids}),
            actionSuccessful: true,
            actionErrorDescription: "",
            stepNumber: step_info.stepNumber + 1
        },

        // update the contract state by using the pure transformation function of the state
        contract_state' = withdraw_pure(contract_state, sender, ids)                                
        }
    }

    // HELPERS
    // ------------------------------------
    // --- Pure Transformation Function ---
    // ------------------------------------
    // 
    // Pure transformation functions: they do not interact at all with the state variables 
    // (only access them through function arguments).


    // When using `pure def`, Quint will check for us if the function is really pure.
    pure def withdraw_pure(c_state: ContractState, sender: Addr, idsList: List[LockupId]): ContractState = {
        // a set is the primary data structure in Quint, and lists are used
        // only when ordering is needed.
        // 
        // Converting here may seem contrived, but:
        //  - if we were first writing the model, we would use sets as a default everywhere
        //  - if we are writing the model after the implementation, we keep the interfaces, but otherwise
        //    translate coding best-practices to modelling best-practices
        val ids = idsList.toSet()
        val new_balance = c_state.contract_balance - 
        ids.fold(
            0,
            (acc, id) => acc + c_state.lockups.get(id).amount
        )
        val new_lockups = c_state.lockups.keys()
                .filter(id => not(contains(ids, id)))
                .mapBy(id => c_state.lockups.get(id))

        {
            free_id: c_state.free_id,
            contract_balance: new_balance,
            lockups: new_lockups
        }
    }
    
    pure def deposit_pure(c_state: ContractState, sender: Addr, amount: int, release_time: Time): ContractState = {
        val new_balance = c_state.contract_balance + amount
        val added_lockup = {            
            amount: amount,
            release_time: release_time,
            owner: sender
        }
        
        // the contract state we want to return here: a new lockup is added
        {
            free_id: c_state.free_id + 1,
            contract_balance: new_balance,
            lockups: c_state.lockups.put(c_state.free_id, added_lockup)            
        }
    }


    
    // INVARIANTS
    // invariant that the contract balance never goes below its initial value
    def contract_balance_ok = contract_state.contract_balance >= INIT_CONTRACT_BALANCE

}