pub const PROGRAM_NAME: &str = "eprover";
pub const PVERSION: &str = "3.3.5";
pub const VERSION: &str = PVERSION;
pub const E_NICKNAME: &str = "Countess Grey";
pub const ECOMMITID: &str = "facc36eaf92d70896d830140efc4382df9e8dcdb";
pub const E_URL: &str = "http://www.eprover.org";
pub const STS_MAIL: &str = "schulz@eprover.org";
pub const HO_MAIL: &str = "jasmin.blanchette@gmail.com";

#[must_use]
pub fn version_line() -> String {
    format!("E {VERSION} {E_NICKNAME} ({ECOMMITID})\n")
}

#[must_use]
pub fn footer() -> String {
    format!(
        "Copyright 1998-2026 by Stephan Schulz, {STS_MAIL},\n\
and the E contributors (see DOC/CONTRIBUTORS).\n\
\n\
This program is a part of the distribution of the equational theorem\n\
prover E. You can find the latest version of the E distribution\n\
as well as additional information at\n\
{E_URL}\n\
\n\
This program is free software; you can redistribute it and/or modify\n\
it under the terms of the GNU General Public License as published by\n\
the Free Software Foundation; either version 2 of the License, or\n\
(at your option) any later version.\n\
\n\
Bug reports for the first-order prover should be sent to <{STS_MAIL}>.\n\
Bug reports with respect to the HO-version should be sent to or at least copied to\n\
<{HO_MAIL}>.\n"
    )
}
