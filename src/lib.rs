extern crate futures;
#[cfg(test)] #[macro_use] extern crate mdo;
#[cfg(test)] extern crate futures_cpupool;

pub mod future {
    use futures::future::*;

    pub fn bind<T, E, TF: Future<Item=T, Error=E> + Sized, IFU: IntoFuture<Error = E>, F: FnOnce(T) -> IFU>(m: TF, f: F) -> AndThen<TF, IFU, F> {
        m.and_then(f)
    }

    pub fn ret<T, E>(x: T) -> FutureResult<T, E> {
        ok::<T, E>(x)
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;
    use futures::future::ok;
    use futures_cpupool::CpuPool;
    use super::future::{bind, ret};

    #[test]
    fn future_mdo() {
        let pool = CpuPool::new_num_cpus();

        let get_num = ok::<u32, String>(42);

        let res = mdo! {
            arg =<< get_num;
            ret ret(arg * 2)
        };

        let val = pool.spawn(res);

        assert_eq!(val.wait().unwrap(), 84);
    }
}
