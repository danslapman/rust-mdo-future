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

pub mod stream {
    use futures::{Async, Poll};
    use futures::future::{Future, FutureResult, ok};
    use futures::stream::*;

    /// bind for Stream, equivalent to `m.map(f).flatten()`
    pub fn bind<E, I, U, F>(m: I, f: F) -> Flatten<Map<I, F>>
        where I: Stream<Error = E> + Sized,
              U: Stream<Error = E> + Sized,
              F: FnMut(<I as Stream>::Item) -> U
    {
        m.map(f).flatten()
    }

    pub fn ret<T, E>(x: T) -> WrappedStream<FutureResult<T, E>> {
        new(Some(ok::<T, E>(x)))
    }

    pub fn mzero<T, E>() -> WrappedStream<FutureResult<T, E>> {
        new(None)
    }

    /// A stream that wraps a single future or nothing (representing an empty stream)
    pub struct WrappedStream<F: Future> {
        future: Option<F>,
    }

    pub fn new<F: Future>(future: Option<F>) -> WrappedStream<F> {
        WrappedStream { future: future }
    }

    impl<F: Future> Stream for WrappedStream<F> {
        type Item = F::Item;
        type Error = F::Error;

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            let ret = match self.future {
                None => return Ok(Async::Ready(None)),
                Some(ref mut future) => {
                    match future.poll() {
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Err(e) => Err(e),
                        Ok(Async::Ready(r)) => Ok(r),
                    }
                }
            };
            self.future = None;
            ret.map(|r| Async::Ready(Some(r)))
        }
    }

}

#[cfg(test)]
mod tests {
    use futures_cpupool::CpuPool;
    use std::vec::Vec;
    use futures::{Future, stream, Stream};

    #[test]
    fn future_mdo() {
        use futures::future::ok;
        use super::future::{bind, ret};
        let pool = CpuPool::new_num_cpus();

        let get_num = ok::<u32, String>(42);
        let get_factor = ok::<u32, String>(2);

        let res = mdo! {
            arg =<< get_num;
            fact =<< get_factor;
            ret ret(arg * fact)
        };

        let val = pool.spawn(res);

        assert_eq!(val.wait().unwrap(), 84);
    }

    #[test]
    fn stream_bind() {
        use super::stream::{bind, ret, mzero};

        let l = bind(stream_range(0, 3), move |x| stream_range(x, 3));
        assert_eq!(execute(l), vec![0, 1, 2, 1, 2, 2]);

        let l = bind(stream_range(0, 3),
                     move |x| bind(stream_range(0, 3), move |y| ret(x + y)));
        assert_eq!(execute(l), vec![0, 1, 2, 1, 2, 3, 2, 3, 4]);

        let l = bind(stream_range(1, 11), move |z| {
            bind(stream_range(1, z + 1), move |y| {
                bind(stream_range(1, y + 1), move |x| {
                    bind(if x * x + y * y == z * z {
                             ret(())
                         } else {
                             mzero()
                         },
                         move |_| ret((x, y, z)))
                })
            })
        });
        assert_eq!(execute(l), vec![(3, 4, 5), (6, 8, 10)]);
    }

    #[test]
    fn stream_mdo() {
        use super::stream::{bind, ret, mzero};
        let l = mdo! {
            x =<< stream_range(0, 3);
            ret stream_range(x, 3)
        };
        assert_eq!(execute(l), vec![0, 1, 2, 1, 2, 2]);
        let l = mdo! {
            x =<< stream_range(0, 3);
            y =<< stream_range(0, 3);
            ret ret(x + y)
        };
        assert_eq!(execute(l), vec![0, 1, 2, 1, 2, 3, 2, 3, 4]);
        let l = mdo! {
            z =<< stream_range(1, 11);
            y =<< stream_range(1, z);
            x =<< stream_range(1, y + 1);
            let test = x * x + y * y == z * z;
            when test;
            let res = (x, y, z);
            ret ret(res)
        };
        assert_eq!(execute(l), vec![(3, 4, 5), (6, 8, 10)]);
    }

    #[test]
    fn stream_ignore() {
        use super::stream::{bind, ret};
        let l = mdo! {
            x =<< stream_range(0, 5);
            ign stream_range(0, 2);
            ret ret(x)
        };
        assert_eq!(execute(l), vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
    }

    // Generate a stream from start (inclusive) to end (exclusive)
    fn stream_range(start: u32, end: u32) -> Box<stream::Stream<Item = u32, Error = String> + Send> {
        Box::new(stream::iter_result((start..end).map(Ok::<u32, String>)))
    }

    // execute the stream on a CpuPool and return a future of the
    // collected result
    fn execute<T, S>(s: S) -> Vec<T>
        where T: Send + 'static,
              S: Stream<Item = T, Error = String> + Send + 'static
    {
        let pool = CpuPool::new_num_cpus();
        let v = pool.spawn(s.collect());
        v.wait().unwrap()
    }

}
