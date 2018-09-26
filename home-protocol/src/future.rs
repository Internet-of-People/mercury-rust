use std::iter;

use futures::prelude::*;
use futures::future::{self, loop_fn, Loop};



// Joins futures into a single future waiting for all of them to finish (no matter if succeed or fail)
// and collect their results into a vector in the original order. This function may not error..
// As a simplification, the function performs transformation
// Iter< Future<T,E> > -> Future< Vec<Result<T,E>>, () >
pub fn collect_results<I>(futures_iterator: I)
    -> Box< Future<Item = Vec< Result< <I::Item as IntoFuture>::Item,
                                       <I::Item as IntoFuture>::Error > >,
                   Error = ()> >
where I: IntoIterator,
      I::Item: IntoFuture + 'static,
{
    // Transform futures that they return their index together with their result
    let futures_vec = futures_iterator.into_iter()
        .map( |i| i.into_future() )
        .enumerate()
        .map( |(idx,fut)|
            fut.then( move |res|
                match res {
                    Ok(v)  => Ok(  (idx, v) ),
                    Err(e) => Err( (idx, e) ),
                }
            )
        )
        .collect::<Vec<_>>();

    // All future combinators, including select_all() panics for empty collections, prevent it
    if futures_vec.is_empty()
        { return Box::new( Ok( Vec::new() ).into_future() ); }

    let vec_fut = loop_fn( ( Box::new( iter::empty() ) as Box<Iterator<Item=_>>, futures_vec ),
        |(finished_results, pending_futures)|
        {
            // Wait for a single future to complete
            future::select_all(pending_futures).then( |first_finished_res|
            {
                // Pending got one item shorter, append the (index, result) as finished
                let (completed, pending) = match first_finished_res {
                    Err( ((idx,err), _i, rest) ) => ( Box::new( finished_results.chain( iter::once( (idx, Err(err)) ) ) ) as Box<Iterator<Item=_>>, rest ),
                    Ok( ((idx,item), _i, rest) ) => ( Box::new( finished_results.chain( iter::once( (idx, Ok(item)) ) ) ) as Box<Iterator<Item=_>>, rest ),
                };

                // Continue with next iteration if any pending future is left, break otherwise
                if pending.is_empty() { Ok( Loop::Break(    (completed, pending) ) ) }
                else                  { Ok( Loop::Continue( (completed, pending) ) ) }
            } )
        } )
        .map( |(idx_res_iter, _pending)|
            // We can only sort vectors, so collect resulted loop iterators
            idx_res_iter.collect::<Vec<_>>() )
        .map( |mut idx_res_vec| {
            // Sort vector by ORIGINAL index
            idx_res_vec.sort_unstable_by(
                |&(ref idx1, ref _res1), &(ref idx2, ref _res2)| idx1.cmp(idx2) );
            idx_res_vec
        } )
        .map( |mut sorted_idx_res_vec|
            // Drop indices from vector items, keep result
            sorted_idx_res_vec
                .drain(..)
                .map( |(_idx,item)| item )
                .collect::<Vec<_>>() );
    Box::new(vec_fut)
}



#[cfg(test)]
mod test
{
    use super::*;
    use futures::sync::mpsc;
    use tokio_core::reactor;

    #[test]
    fn test_collect_empty()
    {
        let mut reactor = reactor::Core::new().unwrap();
        let collect_empty = collect_results( iter::empty::<Result<(),()>>() );
        let result = reactor.run(collect_empty).unwrap();
        assert_eq!( result, Vec::new() );
    }


    #[test]
    fn test_collect_few()
    {
        let mut reactor = reactor::Core::new().unwrap();
        let futs = [ Ok(1), Err(2), Ok(3), Err(4) ];
        let collect_fut = collect_results( futs.iter().cloned() );
        let result = reactor.run(collect_fut).unwrap();
        assert_eq!( result, futs );
    }


    #[test]
    fn test_collect_order()
    {
        let (sink,stream) = mpsc::channel(1);
        let mut reactor = reactor::Core::new().unwrap();
        let mut futs = vec![ Box::new( stream.map(|_| (1) ).collect().map(|vec| vec.first().unwrap().clone()) ) as Box<Future<Item=i32,Error=()>>,
                             Box::new( sink.send(42).map(|_| (2)).map_err(|_| ()) ) ];
        let collect_fut = collect_results( futs.drain(..) );
        let result = reactor.run(collect_fut).unwrap();
        assert_eq!( result, [Ok(1), Ok(2)] );
    }
}
