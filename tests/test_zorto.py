"""Tests for the zorto Python package."""

import zorto
from zorto import Config, Page, Section, Site


# -- version ------------------------------------------------------------------


def test_version_returns_string():
    v = zorto.version()
    assert isinstance(v, str)
    assert len(v) > 0


def test_version_is_semver():
    parts = zorto.version().split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts)


# -- load (using the bundled website/) ----------------------------------------


def test_load_returns_site():
    site = zorto.load("website")
    assert isinstance(site, Site)


def test_load_default_root_fails():
    """Loading from repo root (no config.toml) should raise."""
    try:
        zorto.load(".")
        assert False, "Expected RuntimeError"
    except RuntimeError:
        pass


# -- Site ---------------------------------------------------------------------


def test_site_repr():
    site = zorto.load("website")
    r = repr(site)
    assert "Site(" in r


def test_site_config():
    site = zorto.load("website")
    assert isinstance(site.config, Config)


def test_site_sections():
    site = zorto.load("website")
    assert isinstance(site.sections, list)
    assert len(site.sections) > 0
    assert all(isinstance(s, Section) for s in site.sections)


def test_site_pages():
    site = zorto.load("website")
    assert isinstance(site.pages, list)
    assert all(isinstance(p, Page) for p in site.pages)


# -- Config -------------------------------------------------------------------


def test_config_base_url():
    cfg = zorto.load("website").config
    assert isinstance(cfg.base_url, str)
    assert len(cfg.base_url) > 0


def test_config_title():
    cfg = zorto.load("website").config
    assert isinstance(cfg.title, str)
    assert len(cfg.title) > 0


def test_config_description():
    cfg = zorto.load("website").config
    assert isinstance(cfg.description, str)


def test_config_default_language():
    cfg = zorto.load("website").config
    assert isinstance(cfg.default_language, str)
    assert len(cfg.default_language) > 0


def test_config_theme():
    cfg = zorto.load("website").config
    # theme can be None or str
    assert cfg.theme is None or isinstance(cfg.theme, str)


def test_config_bool_flags():
    cfg = zorto.load("website").config
    assert isinstance(cfg.compile_sass, bool)
    assert isinstance(cfg.generate_feed, bool)
    assert isinstance(cfg.generate_sitemap, bool)
    assert isinstance(cfg.generate_llms_txt, bool)
    assert isinstance(cfg.generate_md_files, bool)


def test_config_repr():
    cfg = zorto.load("website").config
    r = repr(cfg)
    assert "Config(" in r


# -- Section ------------------------------------------------------------------


def test_section_properties():
    sections = zorto.load("website").sections
    assert len(sections) > 0
    s = sections[0]
    assert isinstance(s.title, str)
    assert isinstance(s.path, str)
    assert isinstance(s.permalink, str)
    # description can be None or str
    assert s.description is None or isinstance(s.description, str)


def test_section_pages():
    sections = zorto.load("website").sections
    for s in sections:
        assert isinstance(s.pages, list)
        assert all(isinstance(p, Page) for p in s.pages)


def test_section_len():
    sections = zorto.load("website").sections
    for s in sections:
        assert len(s) == len(s.pages)


def test_section_repr():
    sections = zorto.load("website").sections
    r = repr(sections[0])
    assert "Section(" in r


# -- Page ---------------------------------------------------------------------


def _get_pages():
    """Return all pages from the website, skipping if none exist."""
    pages = zorto.load("website").pages
    if not pages:
        # fall back to section pages
        for s in zorto.load("website").sections:
            pages.extend(s.pages)
    return pages


def test_page_title():
    for p in _get_pages():
        assert isinstance(p.title, str)


def test_page_slug():
    for p in _get_pages():
        assert isinstance(p.slug, str)


def test_page_path():
    for p in _get_pages():
        assert isinstance(p.path, str)


def test_page_permalink():
    for p in _get_pages():
        assert isinstance(p.permalink, str)
        assert p.permalink.startswith("http")


def test_page_content():
    for p in _get_pages():
        assert isinstance(p.raw_content, str)
        assert isinstance(p.content, str)


def test_page_word_count():
    for p in _get_pages():
        assert isinstance(p.word_count, int)
        assert p.word_count >= 0


def test_page_reading_time():
    for p in _get_pages():
        assert isinstance(p.reading_time, int)
        assert p.reading_time >= 0


def test_page_draft():
    for p in _get_pages():
        assert isinstance(p.draft, bool)


def test_page_optional_fields():
    for p in _get_pages():
        assert p.date is None or isinstance(p.date, str)
        assert p.author is None or isinstance(p.author, str)
        assert p.description is None or isinstance(p.description, str)


def test_page_relative_path():
    for p in _get_pages():
        assert isinstance(p.relative_path, str)


def test_page_repr():
    pages = _get_pages()
    if pages:
        r = repr(pages[0])
        assert "Page(" in r


# -- build --------------------------------------------------------------------


def test_build_invalid_root_fails():
    """Building from a non-existent root should raise RuntimeError."""
    try:
        zorto.build("/nonexistent/path")
        assert False, "Expected RuntimeError"
    except RuntimeError:
        pass


# -- public API ---------------------------------------------------------------


def test_all_exports():
    expected = {"build", "Config", "load", "main", "Page", "run_cli", "Section", "Site", "version"}
    assert set(zorto.__all__) == expected
