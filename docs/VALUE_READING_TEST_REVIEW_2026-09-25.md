# Value-reading candidate: confirmed regression-test finding

Reviewed source: `c173e13cc39a6c6bdc3dd2fa1cff7633d54b9bc1`.
Report HEAD before this addendum: `ec1d8e72f4e85b00b9f057c38eaf7015f5798623`;
actual remote main matched that full SHA. Product/source/tests/Cargo/vendor are
unchanged from the candidate. This addendum records one newly identified test
regression; it does not repeat the completed A/B or edit their bound reports.

**Severity: Low**

**Location:** `src/muon.rs`, `tests::value_reading_boundary_fixture`, lines3101–3184,
particularly3171; `value_report`2808; `value_parity_rows`2692;
`value_parity_cases`2686.

**Evidence:** The fixture selects two original episodes per split, so its two O
panels contain four episodes in total. It constructs four two-row U/S streams,
then writes only five lane-final markers with zero calls at3169–3170. At3171 it
expects `value_report(&p)?` to succeed. The current report first calls
`value_parity_rows`, which calls `value_parity_cases`. That function requires
exactly eight parity cases, two per word. Four available O episodes cannot meet
that requirement, so the fixture returns `value reading parity four words`.
It also lacks the parity stream/resolutions and segment start/finish histories
required by the repaired report. Correcting only the first count would therefore
not make its report setup valid.

**Problem:** The production finalizer was strengthened but this directly attached
ignored regression retained its earlier fixture contract. It cannot reach its
report-reread assertions or the later malformed/outside-ID, wrong-model,
missing-resolution and UNKNOWN checks at3172–3184. These are multiple symptoms
of the same stale fixture, not separate product defects.

**Impact:** Running this exact test against the candidate fails on fixture setup;
its later assertions cannot establish regression coverage. This does not show
that those production protections fail. The separate accepted actual-CLI repair
fixture already covers missing/corrupt parity, resolution/start absence,
UNKNOWN/cancellation, undercount, marker mismatch and pending publication.
Existing model raw, diagnostic scores and recorded B parity are unaffected.

**Reproduction:** With the preserved completed first-decision evaluation root in
`R3_FIRST_EVAL_ROOT`, execute the candidate's exact ignored test
`training::fresh::muon::tests::value_reading_boundary_fixture` under its Metal
feature test configuration. Assuming its input files are available, the report
call at3171 necessarily fails because at most four O cases can be selected as
eight parity cases. This finding is established by code and cardinality; that
test was **NOT_RUN in this review**, and no new dynamic PASS is claimed.

**Recommended Fix:** Update this test and its fixture construction. Keep its
small reader/scorer checks separate from full report/finalizer checks; have the
latter reuse a valid parity/segment fixture satisfying the registered report
contract, such as the existing full-report or independent repair fixture. Restore
the valid baseline before each negative mutation. Do not weaken production parity,
usage or UNKNOWN checks to accommodate the old fixture. No patch was applied.

**Necessary verification:** Run the corrected exact test with a nonzero test
count; confirm successful report reread creates no files, and that wrong-model,
missing-resolution and UNKNOWN mutations each reach and fail at their intended
boundary. Reuse the already accepted numeric/model/scorer evidence. No new model
training, GPU numeric suite, teacher or B generation is needed for this finding.

## Reused execution and diagnostic evidence

The candidate `src/muon.rs` hash is
`1d6fcc81fdde7718203c13b2aaef4a5827625de1cc069a161c7dac6b5ab24c4a`.
The actual executor matches
`bbedb487c576e6c614cffaa2944f52c236d4de56407e320ec18d76b2b40cafb8`.
The v2 plan, independent A/B receipts, repaired CLI log, final B readback and
independent score hashes match the preserved report. A receipt:
`66ce75ef25d1eb92a5015e8e605158a753f13a47807abea7095964b4830e2d01`;
B receipt: `c7bfdafb5f143ef0fa6e5c8f83e2a4ebd38bc4e256ed23f69562d91c6b5dc717`.
No completed observation was repeated. This review made zero model calls and
wrote only this report; original source/tests/models/corpus/raw were preserved.

Existing diagnostic A/B completion remains historical accepted evidence. U
whole-value counts train16/48 and dev12/48 equal O; S has train8/48 and dev10/48,
with FULL0/48 on both. S also changes record count, length and position. These
observations do not establish an original-dev improvement, product selection
success or a specific internal cause. Original C word29/192, original C/W quality
FAIL, W non-adoption and S4/S5/S6/Goal1 nonacceptance remain unchanged.

The previously accepted storage amendment is160MiB; build growth/whole peak remain
UNKNOWN. This addendum does not create a fresh cost or quality PASS. It is the
only new publication for the newly found test issue. Its containing Git commit
is the report SHA, distinct from the reviewed source; the actual remote equality
is checked after publication.
