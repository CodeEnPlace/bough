from datetime import date
from src import childs_day


def test_mondays_child_is_fair_of_face():
    assert childs_day(date(2026, 2, 23)) == "fair of face"
