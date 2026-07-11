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
This program is distributed in the hope that it will be useful,\n\
but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n\
GNU General Public License for more details.\n\
\n\
You should have received a copy of the GNU General Public License\n\
along with this program (it should be contained in the top level\n\
directory of the distribution in the file COPYING); if not, write to\n\
the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n\
Boston, MA  02111-1307 USA\n\
\n\
We welcome bug reports and even reasonable questions. If the prover\n\
behaves in an unexpected way, please include the following\n\
information:\n\
\n\
- What did you observe?\n\
- What did you expect?\n\
- The output of `eprover --version`\n\
- The full commandline that lead to the unexpected behaviour\n\
- The input file(s) that lead to the unexpected behaviour\n\
\n\
Most bug reports should be send to <{STS_MAIL}>. Bug reports with \n\
respect to the HO-version should be send to or at least copied to \n\
<{HO_MAIL}>. Please remember that this is an unpaid\n\
volunteer service.\n\
\n\
The original copyright holder can be contacted via email or as\n\
\n\
Stephan Schulz\n\
DHBW Stuttgart\n\
Fakultaet Technik\n\
Informatik\n\
Lerchenstrasse 1\n\
70174 Stuttgart\n\
Germany\n\
\n"
    )
}

#[cfg(test)]
mod tests {
    use super::footer;

    #[test]
    fn footer_preserves_full_c_reporting_and_contact_blocks() {
        let rendered = footer();

        assert!(rendered.contains("WITHOUT ANY WARRANTY"));
        assert!(rendered.contains("- The output of `eprover --version`"));
        assert!(rendered.contains("Bug reports with \nrespect to the HO-version"));
        assert!(rendered.ends_with("70174 Stuttgart\nGermany\n\n"));
    }
}
