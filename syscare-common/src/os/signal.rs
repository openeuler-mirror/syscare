use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;
pub use signal_hook::consts::*;

lazy_static! {
    static ref SIGNAL_FLAG_MAP: RwLock<HashMap<i32, Arc<AtomicBool>>> =
        RwLock::new(HashMap::with_capacity(16));
}

#[inline(always)]
fn setup_signal_handler(signals: &[i32], default: bool) -> std::io::Result<()> {
    let mut signal_flag_map = SIGNAL_FLAG_MAP.write().unwrap();

    for signal in signals {
        match signal_flag_map.contains_key(signal) {
            false => {
                let condition = Arc::new(AtomicBool::new(default));

                signal_flag_map.insert(*signal, condition.clone());
                signal_hook::flag::register_conditional_default(*signal, condition)?;
            }
            true => continue,
        }
    }

    Ok(())
}

#[inline(always)]
fn get_signal_flags(signals: &[i32]) -> Vec<Arc<AtomicBool>> {
    let signal_flag_map = SIGNAL_FLAG_MAP.read().unwrap();

    signals
        .iter()
        .filter_map(|signal| signal_flag_map.get(signal))
        .cloned()
        .collect::<Vec<_>>()
}

#[inline(always)]
fn modify_signal_flags<'a, I>(flags: I, val: bool)
where
    I: IntoIterator<Item = &'a Arc<AtomicBool>>,
{
    for flag in flags {
        flag.store(val, std::sync::atomic::Ordering::Relaxed)
    }
}

pub fn block(signals: &[i32]) -> std::io::Result<()> {
    setup_signal_handler(signals, true)?;

    modify_signal_flags(&get_signal_flags(signals), false);

    Ok(())
}

pub fn unblock(signals: &[i32]) {
    modify_signal_flags(&get_signal_flags(signals), true);
}

pub fn unblock_all() {
    modify_signal_flags(SIGNAL_FLAG_MAP.read().unwrap().values(), true)
}
