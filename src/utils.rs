pub fn filter_log<T, E, F: Fn(E)>(err_cb: F) -> impl Fn(Result<T, E>) -> Option<T> {
    move |res| match res {
        Ok(r) => Some(r),
        Err(e) => {
            (err_cb)(e);
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub trait EitherOps: Sized {
    fn left<R>(self) -> Either<Self, R> {
        Either::Left(self)
    }
    fn right<L>(self) -> Either<L, Self> {
        Either::Right(self)
    }
}

impl<T> EitherOps for T where T: Sized {}

impl<L, R, I> Iterator for Either<L, R>
where
    L: Iterator<Item = I>,
    R: Iterator<Item = I>,
{
    type Item = I;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(l) => l.next(),
            Either::Right(r) => r.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Either::Left(l) => l.size_hint(),
            Either::Right(r) => r.size_hint(),
        }
    }
}
