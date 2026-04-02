// use derive_more::Display;
// use thiserror::Error;

// #[non_exhaustive]
// #[derive(Error, Debug, Display)]
// enum ParseErrType {
//     NoDomainErr,
//     InvalidSequenceErr,
//     MissingDataErr,
// }
// #[non_exhaustive]
// #[derive(Error, Debug, Display)]
// enum WebErr {
//     ConnectionErr(u32), //contains the status of the failed or refused connection
//     ParsingErr(ParseErrType, String), // contains the string that couldnt be parsed
//     LogicErr,           //Encountered some state that does not make sense
// }

// // impl From<io::Error> for CliError {
// //     fn from(error: io::Error) -> Self {
// //         CliError::IoError(error)
// //     }
// // }
