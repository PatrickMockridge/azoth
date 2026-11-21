"""The pure-Python reference implementations of the ``pvt.*`` ids.

Each module is the twin of one Rust kernel in ``crates/azoth-pvt/src/``. The fluid is built
here through :func:`azoth.characterization.mixture_from_characterization` and flashed through
:func:`azoth.eos.reference.pt_flash` or :func:`azoth.eos.reference.tp_multiflash`; the
experiments' composition ledgers are the only state they carry.
"""
