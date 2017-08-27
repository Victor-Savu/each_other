use std::ops::RangeFrom;
use fsm::{State, ContinuationSet};
use meta::sum::{Either, Sum};
use meta::prod::Prod;

pub trait GenSuspend {
    type Gen: Generator;
    type Output: Prod<Left = <Self::Gen as Generator>::Yield, Right = Self::Gen>;
}

pub trait Generator: Sized {
    type Yield;
    type Return;
    type Transition: GenSuspend<Gen = Self> + Sum<Left = <Self::Transition as GenSuspend>::Output, Right = Self::Return>;

    fn next(self) -> Self::Transition;
}

pub enum GenResult<Coro>
    where Coro: Generator
{
    Yield(Coro::Yield, Coro),
    Return(Coro::Return),
}

impl<Coro> GenSuspend for GenResult<Coro>
    where Coro: Generator
{
    type Gen = Coro;
    type Output = (Coro::Yield, Coro);
}

impl<Coro> Sum for GenResult<Coro>
    where Coro: Generator
{
    type Left = (Coro::Yield, Coro);
    type Right = Coro::Return;

    fn to_canonical(self) -> Either<Self::Left, Self::Right> {
        match self {
            GenResult::Yield(y, c) => Either::Left((y, c)),
            GenResult::Return(r) => Either::Right(r),
        }
    }
}

pub struct GenState<S>(S);

impl<S> Generator for GenState<S>
    where S: State<Input = ()>,
          S::Transition: ContinuationSet<Continue = S, Suspend = !>
{
    type Yield = <<<S::Transition as Sum>::Left as Sum>::Left as Prod>::Left;
    type Return = S::Exit;
    type Transition = GenResult<Self>;

    fn next(self) -> GenResult<Self> {
        match self.0.send(()).to_canonical() {
            Either::Left(cont) => {
                let ei: Either<<S::Transition as ContinuationSet>::Head, !> = cont.to_canonical();
                let (y, c) = match ei {
                        Either::Left(l) => l,
                    }
                    .to_canonical();
                GenResult::Yield(y, GenState(c))
            }
            Either::Right(ret) => GenResult::Return(ret),
        }
    }
}

impl<Idx> Generator for RangeFrom<Idx>
    where Self: Iterator
{
    type Yield = <Self as Iterator>::Item;
    type Return = !;
    type Transition = GenResult<Self>;

    fn next(self) -> GenResult<Self> {
        let mut x = self;
        loop {
            if let Some(y) = Iterator::next(&mut x) {
                break GenResult::Yield(y, x);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn range_from() {
        fn foo(q: i64) -> (usize, Vec<i64>) {
            let mut x = q;
            let mut v = vec![];
            each!(1.. => steps in {
                v.push(x);
                x = if x == 1 {
                    return (steps, v)
                } else if x % 2 == 0 {
                    x / 2
                } else {
                    x * 3 + 1
                };
            })
        }

        let (steps, values) = foo(10);

        assert_eq!(steps, values.len());
        assert_eq!(values, [10, 5, 16, 8, 4, 2, 1]);
    }
}