#[derive(Fail, Debug)]
#[fail(display = "error")]
pub enum ErrorKind {
    #[fail(display = "unsupported markdown tag {}", _0)]
    MarkdownLatexConversion(String),
}

#[derive(Fail, Debug)]
pub enum AppError {
    // #[fail(display = "tectonic {}", _0)]
    // Tectonic(failure::SyncFailure<tectonic::Error>),
    //    #[fail(display = "walkdir")]
    //    Walk(walkdir::Error),
    #[fail(display = "application {}", _0)]
    App(ErrorKind),
    /*
        #[fail(display = "syntect dump {}", _0)]
        SyntectDump(Box<dyn std::error::Error + Send + Sync + 'static>),
        #[fail(display = "syntect loading error {}", _0)]
        SyntectLoading(syntect::LoadingError),
    */
}

/*
 *  Tectonic uses error_chain, while this uses the failure crate,
 *  Which causes failure chain to need some glue.
 *  must call sync to be able to convert
 */
pub trait ResultExt<T, E> {
    fn sync(self) -> Result<T, failure::SyncFailure<E>>
    where
        Self: Sized,
        E: ::std::error::Error + Send + 'static;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn sync(self) -> Result<T, failure::SyncFailure<E>>
    where
        Self: Sized,
        E: ::std::error::Error + Send + 'static,
    {
        self.map_err(failure::SyncFailure::new)
    }
}
