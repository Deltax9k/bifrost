service! {
    rpc AppendEntries(term: u64, leaderId: u64, prev_log_id: u64, prev_log_term: u64, entries: Vec<Vec<u8>>, leader_commit: u64) -> u64; //Err for not success
    rpc RequestVote(term: u64, candidate_id: u64, last_log_id: u64, last_log_term: u64) -> (u64, bool); // term, voteGranted
    rpc InstallSnapshot(term: u64, leader_id: u64, lasr_included_index: u64, last_included_term: u64, data: Vec<u8>, done: bool) -> u64;
}

//TODO: Use higher order macro to merge with rpc service! macro when possible to do this in Rust.
//Current major problem is inner repeated macro will be recognized as outer macro which breaks expand

#[macro_export]
macro_rules! raft {
    (
        $(
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident( $( $arg:ident : $in_:ty ),* ) $(-> $out:ty)* $(| $error:ty)*;
        )*
    ) => {
        raft! {{
            $(
                $(#[$attr])*
                def $smt $fn_name( $( $arg : $in_ ),* ) $(-> $out)* $(| $error)*;
            )*
        }}
    };
    (
        {
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident( $( $arg:ident : $in_:ty ),* ); // No return, no error

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        raft! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            def $smt $fn_name( $( $arg : $in_ ),* ) -> () | ();
        }
    };
    (
        {
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident( $( $arg:ident : $in_:ty ),* ) -> $out:ty; //return, no error

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        raft! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            def $smt $fn_name( $( $arg : $in_ ),* ) -> $out | ();
        }
    };
    (
        {
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident( $( $arg:ident : $in_:ty ),* ) | $error:ty; //no return, error

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        raft! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            def $smt $fn_name( $( $arg : $in_ ),* ) -> () | $error;
        }
    };
    (
        {
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident( $( $arg:ident : $in_:ty ),* ) -> $out:ty | $error:ty; //return, error

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        raft! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            def $smt $fn_name( $( $arg : $in_ ),* ) -> $out | $error;
        }
    };
    (
        {} // all expanded
        $(
            $(#[$attr:meta])*
            def $smt:ident $fn_name:ident ( $( $arg:ident : $in_:ty ),* ) -> $out:ty | $error:ty;
        )*
    ) => {
        use std;
        use byteorder::{ByteOrder, LittleEndian};
        use bincode::{SizeLimit, serde as bincode};

        mod sm_args {
            $(
                #[derive(Serialize, Deserialize, Debug)]
                pub struct $fn_name {
                    $(pub $arg:$in_),*
                }
            )*
        }
        mod sm_returns {
            $(
                #[derive(Serialize, Deserialize, Debug)]
                pub enum $fn_name {
                    Result($out),
                    Error($error)
                }
            )*
        }
        pub trait StateMachine {
           $(
                $(#[$attr])*
                fn $fn_name(&self, $($arg:$in_),*) -> std::result::Result<$out, $error>;
           )*
           fn snapshot(&self) -> Vec<u8>;
//           fn dispatch(&self, fn_id: u64, &data: Vec<u8>) -> Option<Vec<u8>> {
//                match fn_id as usize {
//                    $(hash_ident!($fn_name) => {
//                        let decoded: sm_args::$fn_name = bincode::deserialize(&data).unwrap();
//                        let f_result = self.$fn_name($(decoded.$arg),*);
//                        let s_result = match f_result {
//                            Ok(v) => sm_returns::$fn_name::Result(v),
//                            Err(e) => sm_returns::$fn_name::Error(e)
//                        };
//                        Some(bincode::serialize(&s_result, SizeLimit::Infinite).unwrap())
//                    }),*
//                    _ => {
//                        println!("Undefined function id: {}", fn_id);
//                        None
//                    }
//                }
//           }
        }
    };
}

#[cfg(test)]
mod syntax_test {
    raft! {
        def qry get (key: u64) -> String | ();
        def cmd test(a: u32, b: u32) -> bool;
    }
}

