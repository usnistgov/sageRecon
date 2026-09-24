# Third-Party Licenses

This project incorporates and derives from the following third-party
open-source software. Each is used in compliance with its original
license, reproduced in full below.

---

## Sage

Source: https://github.com/lazear/sage
License: MIT

**Redistribution notice:** Sage is linked into the `recon` binary as a Rust
library. The crates `sage-core`, `sage-cli` and `sage-cloudpath` are Cargo git
dependencies pinned to tag `v0.15.0-beta.2`, git commit
`df9219951cc9a54cf4cd55d76541af24b687bd3d`, on `github.com/lazear/sage`
(see `recon-tool/Cargo.toml` and `recon-tool/Cargo.lock`). No modifications
are made to the Sage source. Every `recon` release binary therefore contains
Sage in object form, and the MIT license text below applies to it.

Corrected 2026-09-24: this section said recon "invokes an unmodified Sage
binary". No separate Sage binary has existed since Sage became a library
dependency on 2026-09-01.

MIT License

Copyright (c) 2022 Michael Lazear

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## mzSniffer

Source: https://github.com/wfondrie/mzsniffer
License: Apache License, Version 2.0

Copyright 2023 William E. Fondrie

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Note: portions of this project's polymer-detection logic are ported/adapted
from mzSniffer. Modified files carry a notice indicating they have been
changed from the original, per Apache License 2.0 Section 4(b).

The full Apache License 2.0 text is reproduced once, in "Apache License,
Version 2.0: full text" at the end of this file.

---

## mzdata

Source: https://github.com/mobiusklein/mzdata
License: Apache License, Version 2.0
Version: 0.65.5 (crates.io; pinned in `recon-tool/Cargo.lock`)
Author: Joshua Klein

`recon` links mzdata as a Rust library to read mzML files. It is used
unmodified. The crate's own `LICENSE` file is the unmodified Apache 2.0 text
and names no copyright holder; the crate ships no `NOTICE` file.

The full Apache License 2.0 text is reproduced once, in "Apache License,
Version 2.0: full text" at the end of this file.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

---

## MetaMorpheus (curated modification list)

Source: https://github.com/smith-chem-wisc/MetaMorpheus
License: MIT
Snapshot: clone at commit `7e453540`, taken 2026-08-25.

**Redistribution notice:** this software compiles four data files from
MetaMorpheus into the `recon` binary, from `MetaMorpheus/EngineLayer/Mods/`:

| file | state |
|---|---|
| `aListOfmods.txt` | unmodified |
| `ProteaseMods.txt` | unmodified |
| `surfactants.txt` | unmodified |
| `Mods.txt` | **MODIFIED — see below** |

Verified 2026-09-01 by byte comparison against the clone, after normalising
CRLF to LF. The first three are identical to upstream. `Mods.txt` is not.

**Modifications to `Mods.txt`,** made 2026-08-26 and 2026-08-27, documented in a
comment block at the head of the file itself:

- Two entries renamed and one Unimod cross-reference corrected against Unimod:
  `Glu to PyroGlu` -> `Gln->pyro-Glu`, and `Water Loss` ->
  `Water Loss (Glu->pyro-Glu)` with `DR` changed from Unimod 23 to Unimod 27.
- Five entries with an ambiguous bare `PP   N-terminal.` resolved to either
  `Protein N-terminal.` or `Peptide N-terminal.`.
- Five entries ADDED that are not in upstream: `Met-loss`,
  `Met-loss+Acetylation`, `Met-loss+Methylation`, `Met-loss+Succinylation`,
  `Met-loss+Myristoylation`.

No mass or acceptor residue in an upstream entry was changed. MIT permits
modification and redistribution; this notice records what was changed, and the
MIT text below applies to the original files.

MIT License

Copyright (c) 2016 Stefan Solntsev, Craig D. Wenger

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

## Unimod

Source: https://www.unimod.org/
License: **Design Science License** (Michael Stutz, 1999-2001). The file's own
notice names it; unimod.org publishes the text at https://www.unimod.org/dsl.txt
and describes it in plain language as "a public domain database, distributed
under a copyleft licence". Those are the same thing, not two competing claims —
the DSL is a copyleft licence. Full text reproduced below, as the DSL requires.
Snapshot: `unimod.xml`, schema `majorVersion="2" minorVersion="0"`, 1560 `<umod:mod>`
records. A dated snapshot, unmodified.

**Redistribution notice:** this software uses `unimod.xml` unmodified. The file
carries its own notice, reproduced verbatim from the comment block at the head
of the XML:

    Copyright (C) 2002-2006 Unimod; this information may be copied, distributed
    and/or modified under certain conditions, but it comes WITHOUT ANY WARRANTY;
    see the accompanying Design Science License for more details

**What the DSL requires of us, read from Section 3 below:**

1. **A copy of the License must be distributed along with the Work.** That is why
   the full text is reproduced below, and why this file must ship inside every
   release archive rather than living only in the repository.
2. **Full copyright notice and disclaimer of warranty must be conspicuously
   published on all copies.** The notice above satisfies this.
3. ⚠ **If `unimod.xml` is compiled into the `recon` binary, ship the XML file in
   the release archive as well.** Section 3 permits distributing the Object Form
   only if the Source Data is included in the same distribution (3a), or a
   written offer for it accompanies the distribution (3b). Embedding the bytes
   inside an executable is not obviously "including the Source Data", so ship the
   file too. This costs nothing and removes the question.

**What the DSL does NOT do:** Section 3's aggregation clause states that
combining the Work with other works not based on it does not bring those works
into the scope of the License. `recon`'s own source is not a derivative of the
Unimod database, so the DSL does not propagate to this project's code.

The text below was transcribed from https://www.unimod.org/dsl.txt on
2026-09-01. The DSL itself permits verbatim copying of the document in any
medium.

---

DESIGN SCIENCE LICENSE

TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION

Copyright (C) 1999-2001 Michael Stutz <stutz@dsl.org>
Verbatim copying of this document is permitted, in any medium.

0. PREAMBLE.

Copyright law gives certain exclusive rights to the author of a work,
including the rights to copy, modify and distribute the work (the
"reproductive," "adaptative," and "distribution" rights).

The idea of "copyleft" is to willfully revoke the exclusivity of those
rights under certain terms and conditions, so that anyone can copy and
distribute the work or properly attributed derivative works, while all
copies remain under the same terms and conditions as the original.

The intent of this license is to be a general "copyleft" that can be
applied to any kind of work that has protection under copyright. This
license states those certain conditions under which a work published
under its terms may be copied, distributed, and modified.

Whereas "design science" is a strategy for the development of
artifacts as a way to reform the environment (not people) and
subsequently improve the universal standard of living, this Design
Science License was written and deployed as a strategy for promoting
the progress of science and art through reform of the environment.

1. DEFINITIONS.

"License" shall mean this Design Science License. The License applies
to any work which contains a notice placed by the work's copyright
holder stating that it is published under the terms of this Design
Science License.

"Work" shall mean such an aforementioned work. The License also
applies to the output of the Work, only if said output constitutes a
"derivative work" of the licensed Work as defined by copyright law.

"Object Form" shall mean an executable or performable form of the
Work, being an embodiment of the Work in some tangible medium.

"Source Data" shall mean the origin of the Object Form, being the
entire, machine-readable, preferred form of the Work for copying and
for human modification (usually the language, encoding or format in
which composed or recorded by the Author); plus any accompanying
files, scripts or other data necessary for installation, configuration
or compilation of the Work.

(Examples of "Source Data" include, but are not limited to, the
following: if the Work is an image file composed and edited in PNG
format, then the original PNG source file is the Source Data; if the
Work is an MPEG 1.0 layer 3 digital audio recording made from a WAV
format audio file recording of an analog source, then the original WAV
file is the Source Data; if the Work was composed as an unformatted
plaintext file, then that file is the Source Data; if the Work was
composed in LaTeX, the LaTeX file(s) and any image files and/or custom
macros necessary for compilation constitute the Source Data.)

"Author" shall mean the copyright holder(s) of the Work.

The individual licensees are referred to as "you."

2. RIGHTS AND COPYRIGHT.

The Work is copyrighted by the Author. All rights to the Work are
reserved by the Author, except as specifically described below. This
License describes the terms and conditions under which the Author
permits you to copy, distribute and modify copies of the Work.

In addition, you may refer to the Work, talk about it, and (as
dictated by "fair use") quote from it, just as you would any
copyrighted material under copyright law.

Your right to operate, perform, read or otherwise interpret and/or
execute the Work is unrestricted; however, you do so at your own risk,
because the Work comes WITHOUT ANY WARRANTY -- see Section 7 ("NO
WARRANTY") below.

3. COPYING AND DISTRIBUTION.

Permission is granted to distribute, publish or otherwise present
verbatim copies of the entire Source Data of the Work, in any medium,
provided that full copyright notice and disclaimer of warranty, where
applicable, is conspicuously published on all copies, and a copy of
this License is distributed along with the Work.

Permission is granted to distribute, publish or otherwise present
copies of the Object Form of the Work, in any medium, under the terms
for distribution of Source Data above and also provided that one of
the following additional conditions are met:

(a) The Source Data is included in the same distribution, distributed
under the terms of this License; or

(b) A written offer is included with the distribution, valid for at
least three years or for as long as the distribution is in print
(whichever is longer), with a publicly-accessible address (such as a
URL on the Internet) where, for a charge not greater than
transportation and media costs, anyone may receive a copy of the
Source Data of the Work distributed according to the section above; or

(c) A third party's written offer for obtaining the Source Data at no
cost, as described in paragraph (b) above, is included with the
distribution. This option is valid only if you are a non-commercial
party, and only if you received the Object Form of the Work along with
such an offer.

You may copy and distribute the Work either gratis or for a fee, and
if desired, you may offer warranty protection for the Work.

The aggregation of the Work with other works that are not based on the
Work -- such as but not limited to inclusion in a publication,
broadcast, compilation, or other media -- does not bring the other
works in the scope of the License; nor does such aggregation void the
terms of the License for the Work.

4. MODIFICATION.

Permission is granted to modify or sample from a copy of the Work,
producing a derivative work, and to distribute the derivative work
under the terms described in the section for distribution above,
provided that the following terms are met:

(a) The new, derivative work is published under the terms of this
License.

(b) The derivative work is given a new name, so that its name or title
cannot be confused with the Work, or with a version of the Work, in
any way.

(c) Appropriate authorship credit is given: for the differences
between the Work and the new derivative work, authorship is attributed
to you, while the material sampled or used from the Work remains
attributed to the original Author; appropriate notice must be included
with the new work indicating the nature and the dates of any
modifications of the Work made by you.

5. NO RESTRICTIONS.

You may not impose any further restrictions on the Work or any of its
derivative works beyond those restrictions described in this License.

6. ACCEPTANCE.

Copying, distributing or modifying the Work (including but not limited
to sampling from the Work in a new work) indicates acceptance of these
terms. If you do not follow the terms of this License, any rights
granted to you by the License are null and void. The copying,
distribution or modification of the Work outside of the terms
described in this License is expressly prohibited by law.

If for any reason, conditions are imposed on you that forbid you to
fulfill the conditions of this License, you may not copy, distribute
or modify the Work at all.

If any part of this License is found to be in conflict with the law,
that part shall be interpreted in its broadest meaning consistent with
the law, and no other parts of the License shall be affected.

7. NO WARRANTY.

THE WORK IS PROVIDED "AS IS," AND COMES WITH ABSOLUTELY NO WARRANTY,
EXPRESS OR IMPLIED, TO THE EXTENT PERMITTED BY APPLICABLE LAW,
INCLUDING BUT NOT LIMITED TO THE IMPLIED WARRANTIES OF MERCHANTABILITY
OR FITNESS FOR A PARTICULAR PURPOSE.

8. DISCLAIMER OF LIABILITY.

IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
IN ANY WAY OUT OF THE USE OF THIS WORK, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.

END OF TERMS AND CONDITIONS

---

## UniProtKB (sample FASTA)

Source: https://www.uniprot.org/
License: Creative Commons Attribution 4.0 International (CC BY 4.0),
https://creativecommons.org/licenses/by/4.0/

**Redistribution notice:** `examples/uniprot_sprot_iso_human-2018_06.fasta`,
shipped in this repository as sample data for the Quick Start example, is the
UniProtKB SwissProt + SwissProt varsplic database, *Homo sapiens*, June 2018
release, unmodified. This is the exact snapshot used in the original
analysis of the NIST Candidate RM 8461 human liver reference material (Davis,
Kilpatrick, Ellisor & Neely, *Scientific Data* 6, 324, 2019,
doi:10.1038/s41597-019-0336-7); it is not the current canonical UniProt human
proteome, and a new search should generally use today's snapshot instead.
Cite UniProt: The UniProt Consortium, *Nucleic Acids Research*,
https://www.uniprot.org/help/publications.

---

## Apache License, Version 2.0: full text

Applies to mzSniffer and mzdata above. Copied verbatim from the `LICENSE`
file of the mzdata 0.65.5 crate.

```text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

   Copyright [yyyy] [name of copyright owner]

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```
