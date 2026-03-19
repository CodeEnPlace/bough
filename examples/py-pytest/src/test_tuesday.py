from datetime import date
from src import childs_day


def test_tuesdays_child_is_full_of_grace():
    assert childs_day(date(2026, 2, 24)) == "full of grace"
