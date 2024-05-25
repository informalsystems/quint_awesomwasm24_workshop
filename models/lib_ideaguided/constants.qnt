module constants {
    val INIT_CONTRACT_BALANCE = 1000000
    val SENDERS = Set("user_a", "user_b", "user_c")
    val AMOUNTS = Set(10000, 20000, 30000, 40000, 50000)
    val MAX_LOCKUPS_TO_WITHDRAW = 5
    
    val SPECIAL_ID_VALUE = 0
    val INIT_TIME = 0

    // this one is copied from the contract
    val LOCK_PERIOD = 60 * 60 * 24

    // also taken from the contract
    val INIT_FREE_ID = 1

}