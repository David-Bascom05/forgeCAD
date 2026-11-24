# FEA: prior art, feature tiers, and what it would take

FEA-RESEARCH, 2026-11-24, from origin/main db5c075. Research only: no code. The question is the
owner's: "My experience is with Ansys, and that tool imo is hella slow and buggy. I want to know
what is possible, what it would take to reach feature parity with the best ones, and how hard this
is compared to what else we have done."

Conventions:
- **[E]** marks an estimate. It is a judgement, not a measurement.
- **UNKNOWN** means no public source was found. No benchmark number below is invented.
- Vendor claims are labelled as vendor claims.
- Each source is given as a URL with the claim it supports.
- Repository references are `path:line` on origin/main unless a branch is named.

## 0. The answer in one page

1. **What is possible.** Linear elastic statics, modal, buckling and steady or transient thermal
   are mature, well-understood mathematics. They become *reliable* when the mesh is good and the
   element is right (quadratic tets, not linear ones). They become *fast* when the sparse solve is
   good. Everything past that needs Newton on a non-smooth or non-convex problem: plasticity,
   hyperelastic solids, contact, large deformation, explicit dynamics. In every tool, that work is
   where convergence failures, user-tuned stiffnesses and "it depends" live. That difficulty is
   inherent, not accidental. CFD is a separate field with its own solvers and its own decades.
2. **Why Ansys feels slow and buggy.** There are two parts.
   - *Inherent:* meshing arbitrary CAD, nonlinear contact convergence, and direct-solver cost,
     which grows faster than the model in 3D.
   - *Accidental:*
     - the geometry is read into a second kernel, with its own tolerances, before meshing;
     - opaque "program-controlled" defaults;
     - diagnostics that must be switched on before the solve that fails;
     - a project-schematic UI and licence layers between the user and the solver.
   The accidental part is where a new tool can win (§1.4).
3. **Parity with the best.** Full parity is not a realistic goal. Abaqus, Ansys and Nastran
   represent decades of element libraries, material models and contact algorithms, each with a
   verification suite of thousands of cases (Code_Aster alone cites over 4,200; §3).
   **Beating them on the first tiers is realistic.** Those tiers are linear static, modal,
   buckling, thermal and thermo-mechanical on single parts and bonded assemblies. A new tool can
   beat the incumbents there on *reliability and latency*: meshing straight from our own exact
   B-rep, no translation, instant re-solve on a parametric edit, and *computable error bounds*,
   which no mainstream tool shows by default.
4. **Hardest piece.** Robust, quality, feature-conforming tetrahedral meshing of real CAD is the
   hardest piece *on the critical path of tiers 1–5*. **Confirmed**, with one qualification. It
   is not the hardest problem overall: robust frictional contact under large deformation is
   harder, and nothing makes it reliable by construction. Meshing does have a by-construction
   answer in the literature: exact or indirect predicates, envelope-based tet meshing (§1.3).
   This project's strengths, exact predicates and certified geometry, apply directly to it.
5. **Recommendation** (§6). Tier 1 is linear static with quadratic tets on one part, with:
   - a tet mesher built on new exact `orient3d`/`insphere`;
   - the thermoform crate's sparse Cholesky, lifted into a shared numeric crate;
   - an energy-norm error estimate and adaptivity from the start;
   - the NAFEMS LE and MacNeal–Harder cases as the gate.
   Then modal and buckling, thermal and thermo-mechanical, and bonded assemblies. Do not attempt:
   CFD, explicit crash, general frictional contact, fatigue certification, or a GPU-first solver.

## 1. The landscape

### 1.1 Commercial

| tool | what it is best at | sources |
|---|---|---|
| **Ansys Mechanical / MAPDL** | Broad implicit structural, thermal and modal analysis. Sparse direct and PCG solvers with GPU acceleration, which needs FP64-heavy GPUs. Partial pivoting (contact, mixed u-P, joints) is not GPU-accelerated. Public verification manual, but only a "small subset" of the QA library. | GPU requirements: https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/installation/unix_gpureqt.html (sparse solver compute-bound, FP64 needed, partial-pivoting exclusion). VM: https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/ans_vm/Hlp_V_CH1.html (public VM, "small subset"; the full library is a paid service) |
| **Ansys Discovery** | GPU "live" physics that updates as the geometry changes, with a fidelity slider. A Refine stage hands off to Mechanical or Fluent. The vendor warns that precision varies with your GPU. | https://www.ansys.com/products/3d-design/ansys-discovery ; forum note on thin features vs GPU: https://innovationspace.ansys.com/knowledge/forums/topic/discovery-live-thin-geometric-features-and-relative-gpu-adjustments/ |
| **Abaqus (Standard/Explicit)** | Nonlinear and contact. "General contact", with few restrictions on surface types. Explicit for impact and drop. Publishes NAFEMS benchmark results per element. | Contact overview (6.6 docs): https://classes.engineering.wustl.edu/2009/spring/mase5513/abaqus/docs/v6.6/books/usb/pt09ch29s01abo29.html ; LE1 benchmark page: https://docs.software.vt.edu/abaqusv2025/English/SIMACAEBMKRefMap/simabmk-c-le1.htm ; FV52: https://docs.software.vt.edu/abaqusv2025/English/SIMACAEBMKRefMap/simabmk-c-fv52.htm ; shear locking of linear elements: https://ceae-server.colorado.edu/v2016/books/gsa/ch04s01.html |
| **Nastran (MSC / Simcenter)** | Linear dynamics at aerospace scale. SOL 101 static, 103 modes, 105 buckling, 108/111 frequency response, 112 modal transient, 400 implicit nonlinear. | SOL numbers (secondary source; check the vendor QRG): https://vtpasquale.github.io/NASTRAN_CoFE/5._Quick_Reference_Guide/executive_control_entries/SOL ; https://www.eng-tips.com/threads/want-to-know-the-difference-between-these-solution-codes.425277 |
| **COMSOL** | Equation-level multiphysics coupling. Publishes NAFEMS LE1 as a model. | https://doc.comsol.com/6.4/doc/com.comsol.help.models.sme.elliptic_membrane/elliptic_membrane.html |
| **LS-DYNA** | Explicit crash, drop and penetration. Implicit too, with implicit/explicit switching. Not every feature is available in implicit mode. | https://www.ansys.com/products/structures/ansys-ls-dyna ; https://lsdyna.ansys.com/wp-content/uploads/2025/11/230309_ImplicitExplicit_switching_in_LS-DYNA.pdf |
| **Altair OptiStruct** | Nastran-style solver plus topology optimisation. Publishes its NAFEMS and MacNeal–Harder verification openly. | LE1: https://help.altair.com/hwsolvers/os/topics/solvers/os/nafems_test_problem_le1_r.htm ; MH solid patch: https://help.altair.com/hwsolvers/os/topics/solvers/os/elements_macneal_harder_solid_patch_test_verification_r.htm |
| **Altair SimSolid** | "Meshless": external approximations on the CAD geometry, with adaptivity always on. Fast on large assemblies. Accuracy evidence is mostly vendor-hosted, e.g. "within 10%" against classic FEA in a vendor-linked manual, and 1.7% on one blog case. No independent peer-reviewed validation was found. | Whitepaper: https://altair.com/docs/default-source/resource-library/altair_whitepaper_simsolid-technology-overview_10_30_2020.pdf ; accuracy blog: https://altair.com/blog/articles/Comparing-Altair-SimSolid-with-a-FEA-Benchmark ; adaptivity: https://altair.com/blog/articles/Controlling-Altair-SimSolid-solution-accuracy |
| **SOLIDWORKS Simulation** | CAD-integrated, in tiers. Standard: linear static, plus partial fatigue. Professional: frequency, buckling, thermal, drop. Premium: nonlinear, harmonic, random vibration. | https://www.goengineer.com/en-us/blog/comparing-all-solidworks-simulation-packages-basic-to-advanced ; matrix PDF: https://www.solidworks.com/sites/default/filesd10/26docs06/SOLIDWORKS%20Simulation%20Desktop%20Product%20Matrix.pdf ; NAFEMS frequency tutorial: https://help.solidworks.com/2025/English/simconntutorial/c_nafems_frequency_top.htm |
| **Fusion (Autodesk)** | Cloud-solved. Linear static is in the base product. Modal, buckling, nonlinear, thermal (steady only), thermal stress and event simulation are pay-per-solve or an extension. Publishes NAFEMS LE1. | https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/GUID-8CC26683-A6DD-4FDC-80BD-0DC40D7ACAF2.htm ; LE1: https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/GUID-D258911F-A85E-44C3-A510-74585BA992A2.htm ; tiers: https://www.autodesk.com/products/fusion-360/blog/explore-autodesk-fusion-simulation-capabilities-including-fea-thermal-modal-event-and-generative-design-tools-that-help-engineers-validate-designs-earlier/ |
| **Onshape Simulation** | In-CAD linear static and modal only. The model must be fully constrained (with a separate technique for free bodies). Up to 15 modes. No dynamic loads and no damping. | https://cad.onshape.com/help/Content/simulation.htm ; https://cad.onshape.com/help/Content/modal_simulation.htm |
| **nTop** | Native FEA still meshes. The Intact.Simulation add-on is an immersed ("meshing-free") method on implicit geometry; its accuracy and speed claims are the vendor's. | https://www.ntop.com/software/capabilities/simulation/ ; https://intact-solutions.com/intact-simulation-for-ntop/ |

**The pattern.** CAD-integrated tools (Onshape, Fusion, SOLIDWORKS) win on workflow and lose on
depth. They gate tiers by price and still mesh with a general mesher. Analysis-first tools (Ansys,
Abaqus, Nastran) win on depth and lose on the CAD round trip. Nobody ships "exact B-rep → certified
mesh → solve with an error bound" as one product, and that is the opening.

### 1.2 Open source

| project | what it is | licence | source |
|---|---|---|---|
| CalculiX | Abaqus-like input. Tied, node-to-face and face-to-face penalty contact; mortar work. | GPL-2.0+ | https://dhondt.de/ov_calcu.htm |
| Code_Aster | EDF's industrial solver, ~1.2M lines; cites 4,200+ verification test cases; V-series validation manuals | GPL (GPLv3 per third parties); docs historically GNU FDL | https://www.edf.fr/groupe-edf/innover/rd-un-savoir-faire-mondial/nos-offres/nos-logiciels-et-codes-de-calcul/codeaster ; https://www.edf.fr/sites/groupe/files/2024-07/edf-rd-plaquette-code-aster.pdf |
| Elmer | CSC multiphysics | GPL | https://en.wikipedia.org/wiki/List_of_finite_element_software_packages |
| FEniCSx / dolfinx | Variational forms compiled to kernels | LGPL-3.0+ | https://github.com/FEniCS/dolfinx |
| deal.II | Adaptive hp-FEM library | LGPL-2.1+ | https://arxiv.org/pdf/1910.13247 |
| MFEM | High-order, GPU-capable FEM library | BSD | https://arxiv.org/pdf/1911.09220 |
| MOOSE | INL multiphysics framework | LGPL | https://en.wikipedia.org/wiki/List_of_finite_element_software_packages |
| Kratos | Multiphysics; core BSD, applications may differ | BSD-4 (core) | https://github.com/kratosmultiphysics/kratos |
| **Meshers:** Gmsh | CAD-aware via OpenCASCADE | GPL-2+ with exception | https://gmsh.info/ ; paper: https://gmsh.info/doc/preprints/gmsh_paper_preprint.pdf ; licence: https://metadata.ftp-master.debian.org/changelogs//main/g/gmsh/gmsh_4.8.4+ds2-3_copyright |
| TetGen | Constrained Delaunay refinement | AGPL-3+ or commercial | https://wias-berlin.de/software/tetgen/1.5/doc/manual/manual.pdf ; paper (Si 2015, ACM TOMS 41(2)): https://wias-berlin.de/software/tetgen/ |
| Netgen | Advancing front plus Delaunay | LGPL-2.1 | Debian copyright above |
| fTetWild | Envelope-based robust tet meshing | MPL-2.0 | https://wildmeshing.github.io/ftetwild ; https://pypi.org/project/pytetwild |

For us these projects are **references and oracles, not dependencies**. The kernel is pure Rust
with whole-crate Lean/Aeneas targets (research/kernel-merge/report.md on exact-core/explore). GPL
or AGPL code (CalculiX, Code_Aster, TetGen, Gmsh) cannot be linked into it without taking on that
licence. They make good *differential oracles* in a test harness run as separate processes.
MFEM (BSD), dolfinx (LGPL) and fTetWild (MPL) are useful to read for design.

### 1.3 Newer approaches that change "what is possible"

| approach | what it buys | what it costs or where it stands | sources |
|---|---|---|---|
| **Robust tet meshing (TetWild / fTetWild)** | Meshes triangle soups with an envelope tolerance; fTetWild meshes 98.7% of Thingi10k in under 2 minutes (arXiv text) | The boundary is *approximated* within ε, not conforming; features and curved CAD faces are not kept exactly | https://arxiv.org/abs/1908.03581 |
| **Exact-conforming CDT in 3D (Diazzi, Panozzo, Vaxman, Attene 2023)** | Floating point plus *indirect predicates*, no tolerances. 100% success on the 4,408 valid Thingi10k models. The output conforms exactly to the input. Found a gap in TetGen's theory that can fail on valid input even with exact arithmetic. | It gives a CDT, not a quality mesh; refinement for element quality is a further step | https://arxiv.org/abs/2309.09805 |
| **p-FEM / hierarchic models (StressCheck)** | Raises polynomial order on a fixed, coarse mesh. In the energy norm p-convergence is never slower than h; at corner singularities its rate is twice the h-version's (Babuška, Szabó, Katz 1981). Error is estimated from the p-sequence. | Needs curved high-order geometry mapping and good element shapes; the commercial evidence is the vendor's | https://ivo100.oden.utexas.edu/assets/papers/siam82.pdf ; https://www.esrd.com/simulation-technology/brief-history-of-fea/ |
| **IGA (Hughes, Cottrell, Bazilevs 2005)** | Analysis on the exact NURBS geometry | Real CAD is *trimmed* multi-patch B-rep; volumetric parameterisation and trimmed-patch quadrature are still research. Mainstream use is shells. | https://ui.adsabs.harvard.edu/abs/2005CMAME.194.4135H/abstract ; https://www.nafems.org/downloads/North_America/NAFEMS_2020/presentations_post/hughes_-_ut_at_austin_-_isogeometric_analysis_presentation.pdf |
| **Immersed / embedded: CutFEM, SBM, φ-FEM** | A background grid plus geometry; no body-fitted mesh. CutFEM stabilises cut cells with a ghost penalty. SBM avoids cut cells by shifting the boundary condition (Taylor expansion, Nitsche). φ-FEM works on level sets. | Stress *on the boundary*, where failures start, is the weak spot; high order needs care. These are approximate-domain methods. | CutFEM (Burman et al. 2015, IJNME 104(7)) and SBM (Main & Scovazzi 2018, JCP 372), cited in https://arxiv.org/pdf/2006.00872 ; φ-FEM: https://arxiv.org/pdf/2110.05072 |
| **Walk-on-Spheres / Monte Carlo PDE** | No mesh at all. Point evaluation on arbitrary geometry; Walk on Stars adds mixed Neumann/Dirichlet. | Linear elliptic scalar PDEs only. **No peer-reviewed WoS for linear elasticity was found.** Noise falls as 1/√samples. Gradients, i.e. stress, are costlier still. Good for thermal previews, not for stress today. | https://www.cs.cmu.edu/~kmcrane/Projects/MonteCarloGeometryProcessing/ ; https://www.cs.cmu.edu/~kmcrane/Projects/WalkOnStars/index.html |
| **GPU solvers** | Ansys accelerates sparse direct and PCG on FP64-strong GPUs. NVIDIA cuDSS is a GPU sparse direct library, still in preview. | Consumer GPUs have weak FP64, and Ansys excludes some (e.g. L40) from sparse direct. Speed-ups: UNKNOWN (no public numbers retrieved). | Ansys GPU page above; https://docs.nvidia.com/cuda/cudss/index.html |
| **Learned surrogates** | Fast inference after training on a family of designs | NVIDIA's own material: they "complement" solvers and are for exploration. Generalisation outside the training family is the known weak spot. Ansys SimAI accuracy: UNKNOWN (nothing public retrieved). | https://developer.nvidia.com/blog/ai-powered-simulation-tools-for-surrogate-modeling-engineering-workflows-with-siml-ai-and-nvidia-modulus/ |
| **Verified error estimation** | Equilibrated-flux / Prager–Synge estimators give a *guaranteed* upper bound on the energy error, with reliability constant 1. Guaranteed two-sided eigenvalue bounds come from Crouzeix–Raviart plus conforming FEM (Carstensen & Gedicke 2014; Liu & Plum 2025). | Elasticity needs weakly symmetric stress equilibration (more work than Poisson). The bound is for the discretisation; round-off and the solver need their own accounting. | https://arxiv.org/pdf/2001.09102 ; https://arxiv.org/pdf/2008.05816 ; https://www.mathematik.hu-berlin.de/~cc/cc_homepage/download/2014-CC_JG-Guaranteed_lower_bounds_for_eigenvalues.pdf ; https://arxiv.org/abs/2512.23182v1 |

**What actually changes the game for us.** In order:
1. Exact-predicate, conforming 3D meshing. It turns "failed to mesh" from a common outcome into a
   bug.
2. Guaranteed error bounds. They turn "is my mesh fine enough?" from folklore into a number.
3. p-refinement on a coarse quality mesh. It gives accuracy without re-meshing.

Meshless, WoS and surrogates are interesting but none is a stress solver you can trust today.

### 1.4 Why tools like Ansys are slow and buggy: inherent versus accidental

| symptom | inherent | accidental | evidence |
|---|---|---|---|
| "One or more entities failed to mesh" | Meshing arbitrary CAD with small faces, slivers, near-tangencies and thin walls is hard, and quality guarantees in 3D are weak: slivers escape Delaunay refinement's guarantees (Shewchuk 2002) | The model is re-read into a second kernel (SpaceClaim/Discovery), each with its own tolerance. The fixes are manual: Check Geometry, repair extra edges, virtual topology, defeaturing. Forum advice even conflicts (capture curvature on vs off). | Ansys forum threads: https://innovationspace.ansys.com/forum/forums/topic/error-in-meshing-which-saysone-or-more-entities-failed-to-mesh ; https://innovationspace.ansys.com/forum/forums/topic/meshing-error ; Shewchuk: https://people.eecs.berkeley.edu/~jrs/papers/elemj.pdf |
| CAD itself does not mesh reliably even in a kernel's own mesher | Surface meshing of trimmed B-rep is hard | OpenCASCADE leaves 1.56% of ABC patches unmeshed, and about 5% of models fail outright | https://arxiv.org/pdf/2506.05417 |
| Contact will not converge | Contact is a non-smooth constraint; Newton is not globally convergent on it; friction makes it path-dependent | Diagnostics (Newton–Raphson residual plots) are **off by default** and must be on *before* the failing solve. The remedy is hand-tuning normal stiffness and pinball radius. | Ansys best practice: https://ansyshelp.ansys.com/public/Views/Secured/corp/v242/en/wb_sim/ds_contact_best_prac_converge.html ; PADT: https://www.padtinc.com/2012/10/10/overcoming-convergence-difficulties-in-ansys-workbench-mechanical-part-i-using-newton-raphson-residual-information/ ; forum: https://innovationspace.ansys.com/knowledge/forums/topic/are-there-any-tips-for-enhancing-the-convergence-of-a-highly-compressible-foam-with-nonlinear-contact-against-a-rigid-body-the-newton-raphson-force-residual-does-not-converge-also-the-contact-aga |
| Solves get slow as models grow | A sparse direct solve of a 3D mesh fills in superlinearly. Nested dissection on an n-node 3D grid gives O(n^{4/3}) fill and O(n²) work (Lipton, Rose, Tarjan 1979, SIAM J. Numer. Anal. 16(2); a textbook result, not fetched here). The sparse direct solver is compute-bound (Ansys's own docs). | GPU paths need FP64 data-centre cards; consumer GPUs do not help sparse direct | Ansys GPU page above |
| Wrong answers that look right | Linear tets lock in bending (too stiff); quadratic elements are needed for stress | Defaults and element choice are hidden behind "program controlled" | Abaqus: https://ceae-server.colorado.edu/v2016/books/gsa/ch04s01.html ; Schneider et al. 2019 tet vs hex: https://web.uvic.ca/~teseo/profile/publications/tet-vs-hex/Schneider.2019.tet-vs-hex.pdf |
| Slow edit-to-answer loop | — | The project schematic, geometry hand-off and re-mesh of the whole model on every change. Licence check-out. Cloud queues (Fusion solves in the cloud). | Fusion: help page above. Licence and UI overhead are user experience; no benchmark was found (UNKNOWN). |

Several rows of the "accidental" column are exactly what an integrated kernel removes:
- the second kernel;
- the tolerance mismatch;
- re-meshing what did not change;
- diagnostics that are off by default.

The "inherent" column does not go away. It can be **bounded** (error estimates, meshing that
refuses with a reason) and **kept out of scope** (frictional contact).

## 2. Feature tiers

"Best tool" means the strongest *publicly evidenced* capability. "Our opportunity" is relative to
this repository.

| # | tier | physics, elements and solver | meshing | UI and validation | best tool, and its weak spots | our opportunity |
|---|---|---|---|---|---|---|
| 1 | **Linear static** | Small-strain isotropic elasticity. Tet10 (quadratic), with Tet4 only for previews. Sparse Cholesky (SPD). Loads: force, pressure, gravity, bolt preload as a force. Supports: fixed, roller, elastic. | Quality tet10 mesh with curved edges on curved faces; local refinement at fillets and holes | Von Mises, principal stresses and displacement; reaction check; **error estimate per element**. Validation: NAFEMS LE1, LE10, LE11; MacNeal–Harder patch and beam tests; analytic Lamé and Kirsch cases; manufactured solutions. | All tools do this. The weak spots are meshing failures, hidden mesh-convergence error, and singular stress at re-entrant corners shown as if it were real. | Mesh from the exact B-rep. Guaranteed energy-error bound. Singularity flagged, not plotted as truth. Re-solve on a parametric edit in seconds [E]. |
| 2 | **Modal** | Generalised symmetric eigenproblem K φ = λ M φ. Shift-invert Lanczos on the same Cholesky. Rigid-body modes for free parts. | As tier 1 | NAFEMS FV series (R0015: ~30 one-page free-vibration tests); **guaranteed eigenvalue bounds** are possible (§1.3) | Nastran SOL 103 is the reference. Onshape caps at 15 modes. | Two-sided frequency bounds would be a first in a CAD tool [E: none found in the surveyed products] |
| 3 | **Linear buckling** | Geometric stiffness from a tier-1 prestress; eigenproblem (K + λ K_G) φ = 0 | As tier 1; thin walls need many tets through the thickness, or shells | Euler column and plate analytic cases; the load factor with its sign | Nastran SOL 105, Fusion, SOLIDWORKS Pro. The weak spot is linear buckling's non-conservatism for imperfection-sensitive shells (a textbook fact). | Same machinery as tier 2. Low incremental cost. |
| 4 | **Thermal (steady, then transient)** | Scalar Poisson or heat equation. Tet10. Convection and flux boundary conditions. Implicit Euler or Crank–Nicolson in time. | As tier 1 | NAFEMS thermal benchmarks (T-series, paid); analytic slab and cylinder cases | Fusion does steady only. Ansys and COMSOL are mature. | A scalar problem: cheapest tier. A WoS preview is plausible here. |
| 5 | **Thermo-mechanical** | One-way coupling: temperature, then thermal strain, then tier 1. CTE is already in forge-material. | Shared mesh | Thermal-stress analytic cases (constrained bar, thick cylinder) | Ansys and Abaqus are mature; Fusion has "thermal stress" | Nearly free once tiers 1 and 4 exist |
| 6 | **Assemblies, bonded** | Tied contact (mortar or MPC) between non-matching meshes; bolts as beams with preload | Non-conforming interfaces; contact pairs detected geometrically | Interface traction check; patch tests across the interface | Ansys and Abaqus, with automatic contact detection. The weak spot is detection tolerances on real assemblies. | forge-asm and the exact geometry give exact face pairs: detection by construction |
| 7 | **Nonlinear material** | J2 plasticity (return mapping, consistent tangent); hyperelastic (neo-Hookean, Mooney–Rivlin, Ogden) with mixed u-p to avoid volumetric locking | As tier 1, plus remeshing for large strains | NAFEMS NL series (paid); Cook's membrane; necking bar | Abaqus is widely regarded as the reference (opinion sources only). The weak spots are convergence and material calibration. | forge-thermo already has Ogden hyperelasticity, membrane-only; §4 |
| 8 | **Contact (frictionless, then frictional)** | Augmented Lagrangian or mortar; active-set or semismooth Newton; Coulomb friction is path-dependent | Matching refinement on contact faces | Hertz contact (Ansys VM63 is a public Hertz case); NAFEMS contact benchmarks | Abaqus general contact. The weak spot everywhere is convergence and hand-tuned stiffness (§1.4). | Honest refusals with a reason ("these pairs open and close: …"), not silent tuning. Frictional contact: **do not attempt early**. |
| 9 | **Large deformation** | Total or updated Lagrangian; follower loads; arc-length for snap-through | Remeshing or ALE for extreme cases | Large-deflection cantilever (analytic elastica); snap-through arches | Abaqus and Ansys. The weak spot is limit points without arc-length. | forge-thermo already solves a large-deformation membrane by projected Newton |
| 10 | **Dynamics** | Harmonic (modal superposition, then direct); random (PSD); transient implicit (Newmark/HHT); explicit (central difference, mass scaling) | Explicit needs a uniform minimum element size (the stable time step) | NAFEMS FV forced-vibration cases; SDOF analytic | Nastran (harmonic, random), LS-DYNA (explicit). The weak spots are damping models and explicit run times. | Harmonic and random on top of tier 2 are moderate work. **Explicit crash: do not attempt.** |
| 11 | **Fatigue** | Stress-life and strain-life on linear results; rainflow counting (ASTM E1049, which compiles methods rather than recommending one); mean-stress correction | — | Material S–N data with sources; certification rules are domain-specific | nCode and fe-safe class tools; SOLIDWORKS splits fatigue across tiers. The weak spot is input data quality, not computation. | forge-material's sourced-values discipline fits. Compute is cheap; credible data is not. ASTM: https://store.astm.org/e1049-85r23.html |
| 12 | **CFD** | **A separate world**: Navier–Stokes, turbulence models, boundary-layer meshing, its own V&V | Prism layers, hex-dominant meshes | Its own benchmark suites | Fluent, STAR-CCM+, OpenFOAM | **Do not attempt.** At most an external hand-off. |
| 13 | **Topology optimisation** | SIMP with density filter (Sigmund 2001, 99 lines; Andreassen et al. 2011, 88 lines), on repeated tier-1 solves; MMA or OC updates | A voxel or background grid, not body-fitted | Classic MBB beam and cantilever; mesh independence via the filter | OptiStruct; nTop for lattices. The weak spot is turning the density field back into manufacturable CAD. | With the B-rep kernel we can go back to *editable* CAD. That is a real differentiator, but after tier 6. https://www.topopt.mek.dtu.dk/apps-and-software/efficient-topology-optimization-in-matlab |

## 3. Validation: how a new solver earns trust

### 3.1 Benchmark suites and their availability

| suite | content | licence / availability | what is citable for free |
|---|---|---|---|
| **NAFEMS "The Standard NAFEMS Benchmarks"** (TNSB Rev. 3, Oct 1990; sold as P18) | LE1–LE11 linear elastic (LE1 elliptic membrane, LE10 thick plate, LE11 solid cylinder/taper/sphere thermal), FV free vibration, T thermal, and others | **Paid**: £45 non-member (book or download) at last listing; companion report R0006 £15. https://www.nafems.org/publications/resource_center/p18/ ; https://www.nafems.org/publications/resource_center/r0006/ | The *problem definitions and target values* are re-published in vendors' public verification manuals: Abaqus LE1 (https://docs.software.vt.edu/abaqusv2025/English/SIMACAEBMKRefMap/simabmk-c-le1.htm), Altair OS-V 0010 (https://help.altair.com/hwsolvers/os/topics/solvers/os/nafems_test_problem_le1_r.htm), Ansys VM (https://ansyshelp.ansys.com/public/views/secured/corp/v252/en/ans_vm/Hlp_V_vmfebstale1.html), Fusion and COMSOL (§1.1). Cite the NAFEMS document as the source and those pages as where the numbers can be read. Do not copy the NAFEMS sheets. |
| **NAFEMS free vibration** (R0015 "Selected Benchmarks for Natural Frequency Analysis") | ~30 one-page tests with target frequencies | Paid. https://www.nafems.org/publications/resource_center/r0015/ | FV52 in Abaqus (https://docs.software.vt.edu/abaqusv2025/English/SIMACAEBMKRefMap/simabmk-c-fv52.htm) and Altair (https://help.altair.com/hwsolvers/os/topics/solvers/os/simply_supported_soid_square_plate_fv52_r.htm). Note: the two disagree on a constraint coordinate (−5 m vs −0.5 m); check the geometry. |
| **NAFEMS nonlinear and contact** | NL plasticity, large-deformation and contact benchmarks | Paid; titles not individually confirmed here (UNKNOWN which items) | Vendor manuals re-publish some of them |
| **MacNeal & Harder 1985** | Patch tests; straight, curved and twisted beams; plates; shells; for element accuracy | Journal paper: Finite Elem. Anal. Des. 1:3–20, doi 10.1016/0168-874X(85)90003-4 (Elsevier, paywalled). Definitions are re-published in vendor manuals. | Altair OS-V 0760 (solid patch test) and the straight cantilever: https://help.altair.com/hwsolvers/os/topics/solvers/os/straight_cantilever_beam_verification_r.htm . Known erratum: the twisted beam assumes a curved-edge geometry (2003 comment). |
| **Ansys MAPDL Verification Manual** | Hundreds of VM cases (VM1…), including VM63 static Hertz contact | **Publicly readable**; input listings linked; a "small subset" of the QA library | https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/ans_vm/Hlp_V_CH1.html |
| **Code_Aster V-documents** | Thousands of verification cases with references | GPL code; documentation historically GNU FDL | EDF pages (§1.2) |
| **Analytic solutions** | Lamé thick cylinder, Kirsch plate with a hole, Hertz, Euler and plate buckling, beam modes, heat conduction | Public domain (textbooks: Timoshenko & Goodier; Roark) | Free. Derive and check in SymPy, as research/thermoform/newton/hessian.py already does. |
| **Method of manufactured solutions** | Code verification by observed order of accuracy | Methodology is free. MMS on a commercial elastostatic solver showed 2nd-order displacement convergence. | https://arxiv.org/pdf/1902.07608 ; Sandia order-of-accuracy testing: https://www.osti.gov/servlets/purl/1475255 |

ASME V&V 10 (computational solid mechanics) is the governing standard. It is **paid** and was not
read for this report.

### 3.2 How a new solver earns trust (the ladder)

1. **Code verification.**
   - Patch tests, which must pass to round-off.
   - Manufactured solutions, which must show the theoretical convergence order for each element:
     O(h²) energy norm for Tet10, O(h³) L² displacement.
   - Analytic cases.
   This proves the equations are solved right. It is fully automatable and belongs in CI.
2. **Benchmark agreement.** NAFEMS LE, FV and T, and MacNeal–Harder, at the published target, with
   our error *bound* bracketing the published reference. This is the step that persuades a
   practitioner, because Abaqus, Altair, Ansys, COMSOL and Fusion publish the same table.
3. **Differential testing** against CalculiX or Code_Aster run as separate processes. GPL forbids
   linking, not comparing. Use the same mesh, and test for agreement to solver tolerance.
4. **Validation against measurement.** Strain-gauge or DIC data from published papers. The
   thermoform crate already does this for thickness (research/thermoform/README.md:
   `cargo test -p forge-thermo bench`, digitised profiles), so the habit exists.
5. **Honesty in the output.** Show the error bound. Flag stress singularities at sharp re-entrant
   corners as unbounded. Refuse with a typed reason (as the kernel already does) rather than
   return a plausible wrong number.

## 4. What we already have (origin/main db5c075)

| piece | what is there | carries over | missing for FEA |
|---|---|---|---|
| **forge-thermo** (9.6k lines) | Ogden / neo-Hookean incompressible membrane. Projected Newton with per-triangle Hessians clamped PSD (Teran et al. 2005; Smith, de Goes, Kim 2019). **Sparse supernodal multifrontal Cholesky** on 3×3 node blocks (Liu 1992), nested-dissection order, subtree parallelism, a diagonal shift and a not-PD signal (kernel/forge-thermo/src/newton.rs:1-7, 13-17, 152-160, 519, 593). Sticking contact with a tool. Benchmarked against measured parts (src/bench.rs). | The **3×3-block SPD Cholesky is exactly the structure of 3D vector elasticity**, one block per node. The Newton and line-search pattern, hyperelastic laws, contact-as-stop and the SymPy-checked Hessian practice all carry over. | Everything is `pub(crate)` inside forge-thermo, so it must be lifted out. The dissection splits on **2D reference coordinates** (newton.rs:54-56); volumes need a 3D (or graph) separator. No eigen-solver (Lanczos), no iterative solver or preconditioner, no out-of-core. Elements are membrane triangles only: no Tet4/Tet10, no quadrature on tets. |
| **forge-math** | Vec3, `dmath` (deterministic libm), `par` (scoped threads, results in item order), `linalg` (2×2, small dense QR), `Interval` (outward-rounded, proven in ieee754/), `exact.rs` (dyadic `Dy`, big integers), and `predicates`: `orient2d`, `incircle`, `cross_sign`, `dot3`, `triple`, all sign-exact (predicates.rs:405-667) | Deterministic parallelism (answers independent of thread count) is rare in FEA and valuable for reproducibility. `triple(a,b,c)` is an exact sign of a·(b×c) on *given* vectors. Intervals support verified bounds. | **No `orient3d` or `insphere`.** `triple` on differences (b−a etc.) is not orient3d, because the differences round. A tet mesher needs Shewchuk's adaptive `orient3d` and `insphere` (or Attene's indirect predicates for constructed Steiner points). No sparse matrix type outside forge-thermo, and no eigenvalue routines. |
| **forge-mesh** | `cdt.rs`: 2D constrained Delaunay with exact predicates and Lawson flips; output depends only on the input; a refinement hook driven by chord need (cdt.rs:1-6, 1175-1186). `tessellate.rs`: a **watertight** body mesh, with each edge cut once and shared by both faces (tessellate.rs:856-858), chord bounds proved in research/mesh-bounds/. L4 proofs over the CDT (per the kernel-merge report). | A watertight, exactly shared surface triangulation of our own B-rep is the *input a conforming tet mesher needs*. Most CAD-to-FEA pipelines lose it at a STEP hand-off. | Surface triangles are sized for **chord error, not element quality**: centroid and midpoint insertion, no minimum-angle guarantee (cdt.rs:1175-1180). FEA needs quality-graded surface meshes and a **3D tet mesher** with quality refinement and sliver removal. Nothing 3D exists. Tet10 also needs mid-edge nodes placed *on* the curved surface, with a validity check of the curved element (Johnen, Remacle, Geuzaine 2013, already cited in docs/fold-proof.md:13). |
| **forge-material** | 61 records, each value with a source; `youngs`, `poisson`, `yield`, `uts`, `cte`, density; per-value `kind` (`cited`, `user`, `stand-in`) (lib.rs:1-14) | E, ν, CTE and density are tiers 1–5's material inputs, with provenance. The stand-in flag fits the "honest output" principle. | Thermal conductivity, specific heat, S–N curves, plasticity hardening, temperature dependence |
| **B-rep kernel** (forge-geom 34.7k, forge-topo 7.4k, forge-bool 21.5k + bool-core 3.0k lines) | Exact faces and edges with named selection (`edges(of: boss, convex)`), Krawczyk-certified intersections, typed refusals | Boundary conditions **by name in the part's own text**, e.g. `fix(faces(of: base, bottom))`, survive parametric edits, so re-solve on edit is natural. Exact face identity gives exact contact-pair detection in assemblies. Curved-surface evaluation places Tet10 mid-nodes exactly. | A volume mesher that consumes the B-rep. Mapping results back to faces for queries |
| **forge-gpu** | An experimental CUDA path for boolean classification, with the PTX loaded at run time (lib.rs:1-11) | A precedent for an optional GPU path behind an exact CPU fallback | No GPU linear algebra |
| **forge-brake / CAM checkers** | Certified Clear/Hit/Undecided branch-and-bound (brake bb.rs; CAM on cam/* branches, not on main at db5c075) | The *contract* (certified verdict or Undecided) is the same shape as "this stress is within ±x, guaranteed, or I cannot say" | — |
| **Planned shared numeric core** | research/kernel-merge/report.md (branch exact-core/explore) recommends (b): one dependency-free numeric and 2D core, with one integer, one `Dy`, one exact-real DAG, one interval and one Shewchuk. The name "forge-num" is the coordinator's; the report does not name the crate. | `orient3d` and `insphere` belong in that core next to the one Shewchuk. A sparse SPD solver could be a sibling crate. | The report scopes the core to numbers, exact predicates and 2D. A sparse linear-algebra crate is **not** in it and would be a separate decision. |

**Summary.** We have:
- the hard half of the linear solver: the factorisation;
- the hard half of the meshing input: an exact, watertight surface of our own geometry;
- material data with provenance;
- an existing habit of validating against measurement.

We lack:
- 3D predicates;
- a tet mesher;
- tet elements and quadrature;
- an eigen-solver;
- error estimation;
- a results UI.

## 5. Difficulty compared with what this project has built

Reference points (sizes from research/kernel-merge/report.md and `wc -l` on main):
- thermoform FEM: 9.6k lines;
- bend sim (forge-brake): 5.8k lines;
- CAM exact geometry core: 8.2k lines (on cam/* branches);
- exact booleans: forge-bool 21.5k + bool-core 3.0k lines, with Lean proofs.

The table expresses effort **[E]** in "thermoform units" (TU): one TU is roughly what forge-thermo
took. These are judgements and carry wide error bars.

| tier | effort [E] | risk | the hard parts, honestly |
|---|---|---|---|
| **3D predicates** (`orient3d`, `insphere`, filtered and exact) | 0.1 TU | low | Shewchuk's orient3d and insphere are published and the 2D versions are here already. Proving the fast-path bounds at L4, as was done for orient2d, adds work. |
| **Tet mesher: conforming CDT from our watertight surface, plus quality refinement and sliver removal** | **2–4 TU** | **high** | The single hardest piece of tiers 1–6:<br>- boundary recovery in 3D, which may need Steiner points (Schönhardt polyhedra exist);<br>- TetGen's own theory had a gap (Diazzi et al. 2023);<br>- sliver removal has weak guarantees;<br>- thin walls and small features force tiny elements;<br>- curved Tet10 validity.<br>About as hard as the exact boolean, and it **must be robust on every part the kernel can build**. Recommended route: Diazzi et al.'s CDT with indirect predicates, then refinement and smoothing. |
| **Tier 1, linear static (Tet10)** | 1 TU beyond the mesher | medium | Elements and quadrature are textbook. Lifting the Cholesky out and generalising nested dissection to 3D is moderate. Large models will need an iterative solver (PCG with AMG or p-multigrid), a real project in itself. Singularities must be detected and reported. |
| **Error estimation + adaptivity** | 1 TU | medium | Residual estimators are easy. *Guaranteed* equilibrated bounds for elasticity are research-grade (weakly symmetric stresses) but published. This is the closest analogue of the certified CAM checker. |
| **Tiers 2–3, modal and buckling** | 0.5 TU | low–medium | Shift-invert Lanczos with robust handling of clustered and rigid-body modes. Guaranteed eigenvalue bounds add another 0.5 TU. |
| **Tiers 4–5, thermal and thermo-mechanical** | 0.5 TU | low | Scalar, reuses everything |
| **Tier 6, bonded assemblies** | 1 TU | medium | Mortar tying of non-matching meshes; bolt models |
| **Tier 7, nonlinear material** | 1–2 TU | medium–high | Return mapping and consistent tangents are textbook. Volumetric locking (mixed u-p) and robust load stepping are the hard parts. forge-thermo's Ogden and projected-Newton work is the head start. |
| **Tier 9, large deformation** | 1 TU | medium–high | Arc-length, follower loads, remeshing |
| **Tier 8, contact** | 3+ TU | **very high** | Frictionless is tractable: mortar plus semismooth Newton. Friction is path-dependent and **the commercial tools' known weak spot after decades**. There is no "by construction" fix. This is the hardest *research* problem on the list. |
| **Tier 10, dynamics** | harmonic and random 0.5 TU; implicit transient 0.5 TU; explicit 3+ TU | explicit: high | Explicit needs contact, element erosion and mass scaling: an LS-DYNA-shaped product |
| **Tier 11, fatigue** | 0.5 TU compute | data risk | Credible S–N data with sources is the real cost |
| **Tier 13, topology optimisation** | 1 TU (SIMP) + 2 TU (back to editable B-rep) | medium | Density to B-rep is the differentiating part and is open-ended |
| **Results UI** (contours, probes, deformed shape, bound display) | 1 TU [E] | low–medium | forge-render exists; per-vertex scalar fields and legends are needed |
| **Tier 12, CFD** | out of scope | — | A separate field |

**Is robust volume meshing of real CAD the single hardest piece?**
- **For tiers 1–6: confirmed.** Every tier sits on it, it is where the commercial tools fail
  visibly (§1.4), and it has the highest effort-times-risk of anything before contact.
- **Overall: refuted, narrowly.** Frictional contact under large deformation is harder. Meshing
  can be made correct by construction (exact or indirect predicates, a conforming CDT); contact
  convergence cannot.

**How it compares with the exact boolean.** The boolean needed exact predicates on curved
geometry and a topology that stays valid. A tet mesher needs exact predicates on *linear* geometry
(easier) but in 3D, plus *quality* refinement, where the guarantees are weak. That is a heuristic
fight the boolean did not have [E].

## 6. Recommendation: a phased plan

### Phase 0: groundwork (≈0.5 TU [E])
- Add `orient3d` and `insphere` (filtered, exact, with Big fallbacks) next to the one Shewchuk,
  in the shared numeric core the kernel-merge report proposes.
- Lift the supernodal Cholesky out of forge-thermo into its own crate, with forge-thermo as its
  first client. Keep bits identical: forge-thermo's bench must not change. Generalise the
  dissection to 3D coordinates or a graph.
- Add a Lanczos eigensolver on top of the factorisation.

### Phase 1: tier 1, linear static on one part (≈4–6 TU [E] including the mesher)
- **Mesher:**
  - conforming 3D CDT from `tessellate`'s watertight surface, re-meshed for quality on each face
    (2D CDT with a minimum-angle refinement, a new `Refine` impl);
  - then Delaunay refinement and sliver removal;
  - then Tet10 with mid-nodes on the exact surface and a curved-validity check.
  - Typed refusal with the offending faces when it cannot mesh: never a silent bad mesh.
- **Solver:** Tet10 linear elasticity, sparse Cholesky, and loads and supports by name in the
  `.fg` text, with the same selection language as features.
- **Error:**
  - a residual-type estimator and h-adaptivity from day one;
  - equilibrated guaranteed bounds as the stretch goal of this phase;
  - **flag re-entrant-corner singularities** rather than reporting their peak.
- **Gate:**
  - patch tests;
  - manufactured solutions with observed order;
  - NAFEMS LE1, LE10 and LE11 and MacNeal–Harder at published targets (values read from public
    vendor manuals; NAFEMS cited);
  - Lamé and Kirsch analytic cases;
  - a differential harness against CalculiX as a separate process.
- **Speed:** re-solve on a parameter edit reusing the symbolic factorisation (and the mesh, where
  the edit did not touch that region) [E: plausible target of seconds for 10⁵ DOF on a laptop;
  to be measured, not promised].

### Phase 2: tiers 2–5 (≈2 TU [E])
Modal (with guaranteed eigenvalue bounds as the headline), linear buckling, steady and transient
thermal, and one-way thermo-mechanical. Gate: NAFEMS FV and T, analytic cases.

### Phase 3: tier 6 and a results UI (≈2 TU [E])
Bonded assemblies through exact face pairs from forge-asm (mortar tying, bolt preload as a
force). Contours, probes, and an error-bound overlay in forge-render.

### Phase 4, optional: tiers 7 and 9 (≈2–3 TU [E])
Plasticity and hyperelastic solids with mixed u-p, and large deformation with arc-length.
forge-thermo's experience applies directly.

### Where we can beat Ansys
1. **Mesh robustness by construction.** One kernel, no translation, exact predicates. When meshing
   fails, it fails with the faces named, as the boolean does.
2. **A number for "is the mesh fine enough".** A guaranteed energy-error bound and, for modal,
   two-sided frequency bounds. No surveyed CAD-integrated tool shows a guaranteed bound by
   default [E: based on the products surveyed in §1.1; not exhaustive].
3. **Edit-to-answer latency.** Loads and supports are named in the part's text and survive
   parametric edits. Re-solves reuse the symbolic factorisation. No cloud queue and no licence
   check-out.
4. **Determinism.** Results that do not depend on thread count (forge-math `par`), so a regression
   test is exact.
5. **Honesty.** Typed refusals, singularity flags, and material values with provenance and a
   stand-in flag.

### What NOT to attempt
- **CFD**: a separate field.
- **Explicit crash/impact**: LS-DYNA's world. It needs contact, erosion and its own validation.
- **General frictional contact** before tiers 1–6 are solid. Frictionless contact may follow
  phase 4.
- **Fatigue certification**: provide the compute later if asked, but the data and code rules are
  domain-specific.
- **A GPU-first solver.** Sparse direct on GPUs needs FP64 data-centre cards (Ansys's own
  requirement), and consumer GPUs do not help. A GPU *preview* path in the forge-gpu style (fast
  answer, exact CPU fallback) is reasonable later.
- **Meshless, WoS or learned surrogates as the primary solver.** None is a trustworthy stress
  solver today (§1.3). WoS is worth a look later as an instant *thermal* preview.
- **IGA on trimmed B-rep.** Still research for solids.
- **Linking GPL or AGPL solvers or meshers** (CalculiX, Code_Aster, TetGen, Gmsh): use them only
  as external oracles.

## Sources not fetched in full

Several sources above were read through search-result excerpts, not fetched in full:
- NAFEMS product pages;
- the SimSolid whitepaper;
- vendor verification pages;
- the Diazzi et al. and fTetWild abstracts.

Claims taken from them are limited to what the excerpts state. Lipton–Rose–Tarjan (nested
dissection complexity) and Shewchuk's 3D predicates are cited from standard knowledge and were not
fetched here.
