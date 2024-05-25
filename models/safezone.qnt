module safezone {
    
    import basicSpells.* from "spells/basicSpells"

    // variables
    var x : int    

    // init state
    action init_20 = 
        x' = 20            

    action init = all {
        x' = 21    
    }
        
    // model evolution
    action step =
        nondet force = oneOf(1.to(100))
        any {
            up(force),
            down(force)
        }

    // users' messages
    action up(force: int) : bool = 
        x' = x + 6*force
           
        

    action down(force: int) : bool =
        val new_x = x - 9*force
        all{
            require(new_x >= 0),
            x' = new_x           
        }

    // invariant
    val safe = not(x == 0)

    }