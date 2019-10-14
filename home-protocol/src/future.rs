use std::iter;
//use std::time::Duration;

use futures::future::{self, loop_fn, poll_fn, Loop};
//use futures::stream::StreamFuture;
//use tokio_current_thread as reactor;

use crate::*;

// NOTE recent tokio versions already provide deadlines with implicit default timer
/*
pub struct StreamWithDeadline<S: Stream> {
    inner: StreamFuture<S>,
    timeout: Duration,
    timer: reactor::Timeout,
    handle: reactor::Handle,
}

impl<S: Stream> StreamWithDeadline<S> {
    pub fn new(stream: S, timeout: Duration, handle: &reactor::Handle) -> Self {
        let inner = stream.into_future();
        Self {
            inner,
            timeout,
            handle: handle.clone(),
            timer: reactor::Timeout::new(timeout, handle).unwrap(),
        }
    }
}

impl<S: Stream> Stream for StreamWithDeadline<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.timer.poll() {
            Ok(Async::NotReady) => {}
            Ok(Async::Ready(())) => return Ok(Async::Ready(Option::None)),
            Err(_e) => return Ok(Async::Ready(Option::None)),
        };

        match self.inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),

            Ok(Async::Ready((x, stream))) => {
                self.inner = stream.into_future();
                self.timer = reactor::Timeout::new(self.timeout, &self.handle).unwrap();
                Ok(Async::Ready(x))
            }

            Err((e, stream)) => {
                self.inner = stream.into_future();
                Err(From::from(e))
            }
        }
    }
}
*/

//// Run futures until one is ready, then ignore remaining pending ones and return the first result
//pub fn select_first<I>(futures_iterator: I)
//    -> AsyncResult< <I::Item as IntoFuture>::Item, <I::Item as IntoFuture>::Error >
//where I: IntoIterator,
//      I::Item: IntoFuture + 'static,
//{
//    let fut = future::select_all(futures_iterator)
//        .map( |(done, _idx, _pending)| done )
//        .map_err( |(err, _idx, _pending)| err );
//    Box::new(fut)
//}

// Joins futures into a single future waiting for all of them to finish (no matter if succeed or fail)
// and collect their results into a vector in the original order. This function may not error..
// As a simplification, the function performs transformation
// Iter< Future<T,E> > -> Future< Vec<Result<T,E>>, () >
// TODO consider simplifying this trying to use conventional join_all in the direction of
//      join_all( futs.map( fut.then( Ok(res) ) ) )
//          .map( results.filter_map( res.ok() ) )
pub fn collect_results<I>(
    futures_iterator: I,
) -> AsyncResult<Vec<Result<<I::Item as IntoFuture>::Item, <I::Item as IntoFuture>::Error>>, ()>
where
    I: IntoIterator,
    I::Item: IntoFuture + 'static,
{
    // Transform futures that they return their index together with their result
    let futures_vec = futures_iterator
        .into_iter()
        .map(|i| i.into_future())
        .enumerate()
        .map(|(idx, fut)| {
            fut.then(move |res| match res {
                Ok(v) => Ok((idx, v)),
                Err(e) => Err((idx, e)),
            })
        })
        .collect::<Vec<_>>();

    // All future combinators, including select_all() panics for empty collections, prevent it
    if futures_vec.is_empty() {
        return Box::new(Ok(Vec::new()).into_future());
    }

    let vec_fut = loop_fn(
        (Box::new(iter::empty()) as Box<dyn Iterator<Item = _>>, futures_vec),
        |(finished_results, pending_futures)| {
            // Wait for a single future to complete
            future::select_all(pending_futures).then(|first_finished_res| {
                // Pending got one item shorter, append the (index, result) as finished
                let (completed, pending) = match first_finished_res {
                    Err(((idx, err), _i, rest)) => (
                        Box::new(finished_results.chain(iter::once((idx, Err(err)))))
                            as Box<dyn Iterator<Item = _>>,
                        rest,
                    ),
                    Ok(((idx, item), _i, rest)) => (
                        Box::new(finished_results.chain(iter::once((idx, Ok(item)))))
                            as Box<dyn Iterator<Item = _>>,
                        rest,
                    ),
                };

                // Continue with next iteration if any pending future is left, break otherwise
                if pending.is_empty() {
                    Ok(Loop::Break((completed, pending)))
                } else {
                    Ok(Loop::Continue((completed, pending)))
                }
            })
        },
    )
    .map(|(idx_res_iter, _pending)|
            // We can only sort vectors, so collect resulted loop iterators
            idx_res_iter.collect::<Vec<_>>())
    .map(|mut idx_res_vec| {
        // Sort vector by ORIGINAL index
        idx_res_vec
            .sort_unstable_by(|&(ref idx1, ref _res1), &(ref idx2, ref _res2)| idx1.cmp(idx2));
        idx_res_vec
    })
    .map(|mut sorted_idx_res_vec|
            // Drop indices from vector items, keep result
            sorted_idx_res_vec
                .drain(..)
                .map( |(_idx,item)| item )
                .collect::<Vec<_>>());
    Box::new(vec_fut)
}

// Alternative implementation with poll, just for comparison to the composite solution above
pub fn collect_results2<I>(
    futures_iterator: I,
) -> impl Future<
    Item = Vec<Result<<I::Item as IntoFuture>::Item, <I::Item as IntoFuture>::Error>>,
    Error = (),
>
where
    I: IntoIterator,
    I::Item: IntoFuture + 'static,
{
    // input has all uncompleted futures prefixed with its original position
    // output has all completed results prefixed with the original position of the future from input
    // each poll will remove completed futures from input and append them to output
    let mut input =
        futures_iterator.into_iter().map(|a| a.into_future()).enumerate().collect::<Vec<_>>();
    let mut output = Vec::new();

    poll_fn(move || {
        let x = input
            .drain(..)
            .filter_map(|(idx, mut f)| match f.poll() {
                Ok(Async::NotReady) => Some((idx, f)),
                Ok(Async::Ready(item)) => {
                    output.push((idx, Ok(item)));
                    None
                }
                Err(err) => {
                    output.push((idx, Err(err)));
                    None
                }
            })
            .collect::<Vec<_>>();

        if x.is_empty() {
            output
                .sort_unstable_by(|&(ref idx1, ref _res1), &(ref idx2, ref _res2)| idx1.cmp(idx2));
            let result = output.drain(..).map(|(_idx, f)| f).collect::<Vec<_>>();
            Ok(Async::Ready(result))
        } else {
            input = x;
            Ok(Async::NotReady)
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::sync::mpsc;
    use tokio_current_thread as reactor;

    #[test]
    fn test_collect_empty() {
        let mut reactor = reactor::CurrentThread::new();
        let collect_empty = collect_results(iter::empty::<Result<(), ()>>());
        let result = reactor.block_on(collect_empty).unwrap();
        assert_eq!(result, Vec::new());
    }

    #[test]
    fn test_collect_few() {
        let mut reactor = reactor::CurrentThread::new();
        let futs = [Ok(1), Err(2), Ok(3), Err(4)];
        let collect_fut = collect_results(futs.iter().cloned());
        let result = reactor.block_on(collect_fut).unwrap();
        assert_eq!(result, futs);
    }

    #[test]
    fn test_collect_order() {
        let (sink, stream) = mpsc::channel(1);
        let mut reactor = reactor::CurrentThread::new();
        let mut futs = vec![
            Box::new(stream.map(|_| (1)).collect().map(|vec| *vec.first().unwrap()))
                as AsyncResult<i32, ()>,
            Box::new(sink.send(42).map(|_| (2)).map_err(|_| ())),
        ];
        let collect_fut = collect_results(futs.drain(..));
        let result = reactor.block_on(collect_fut).unwrap();
        assert_eq!(result, [Ok(1), Ok(2)]);
    }
}
