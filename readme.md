# rust-mdo-future

Rust-mdo-future is a small crate that enables future support in [mdo](https://github.com/TeXitoi/rust-mdo)

Just take a look:

```rust
#[macro_use] extern crate mdo;
extern crate mdo-future;

use futures::Future;
use futures::future::ok;
use futures_cpupool::CpuPool;
use mdo-future::future::{bind, ret};

let pool = CpuPool::new_num_cpus();

let get_num = ok::<u32, String>(42);

let res = mdo! {
    arg =<< get_num;
    ret ret(arg * 2)
};

let val = pool.spawn(res);

assert_eq!(val.wait().unwrap(), 84);
```