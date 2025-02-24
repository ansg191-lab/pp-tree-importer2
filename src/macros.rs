macro_rules! trys {
    ($tx:expr_2021, $expr:expr_2021 $(,)?) => {
        match $expr {
            std::result::Result::Ok(val) => val,
            std::result::Result::Err(err) => {
                // Will only fail if rx is closed
                // If rx is closed, we can't do anything anyway
                let _ = $tx.send(Err(err));
                return;
            }
        }
    };
}
pub(crate) use trys;

macro_rules! yield_from {
    ($tx:expr_2021, $expr:expr_2021 $(,)?) => {{
        let mut x = $expr;
        while let Some(result) = x.next().await {
            let val = trys!($tx, result);
            if $tx.send(Ok(val)).is_err() {
                return;
            }
        }
    }};
}
pub(crate) use yield_from;
