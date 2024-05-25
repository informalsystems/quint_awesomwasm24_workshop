// -*- mode: Bluespec; -*-

module types {
    type Addr = str
    type LockupId = int

    type MsgArgs = 
        | NoArgs
        | DepositArgs({sender: Addr, amount: int})
        | WithdrawArgs({sender: Addr, lockup_ids: List[LockupId]})

    type StepInfo = {
        actionTaken: str,
        msgArgs: MsgArgs,
        actionSuccessful: bool,
        actionErrorDescription: str,
        stepNumber: int
        }
    
    
    type Time = int
    type ChainState = {
        time: Time
    }    
    
    type Lockup = {
        owner: Addr,
        amount: int,        
        release_time: Time
    }

    type ContractState = {
        free_id: LockupId,
        lockups: LockupId -> Lockup,
        contract_balance: int
    }
}

