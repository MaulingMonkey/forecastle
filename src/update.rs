use futures::executor::*;
use futures::task::*;

use std::cell::RefCell;
use std::future::Future;


pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    TL.with(|tl|{
        tl.local_spawner.spawn_local(future).unwrap()
    })
}

pub(crate) fn run_local() {
    TL.with(|tl|{
        tl.local_pool.borrow_mut().run_until_stalled()
    })
}



thread_local! {
    static TL : ThreadLocal = ThreadLocal::default();
}

struct ThreadLocal {
    local_pool:     RefCell<LocalPool>,
    local_spawner:  LocalSpawner,
}

impl Default for ThreadLocal {
    fn default() -> Self {
        let local_pool = LocalPool::new();
        let local_spawner = local_pool.spawner();
        Self { local_pool: RefCell::new(local_pool), local_spawner }
    }
}
