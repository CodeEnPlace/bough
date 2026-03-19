from datetime import date


def childs_day(d: date) -> str:
    """Return the Monday's Child verse for a given date.

    >>> childs_day(date(2026, 2, 25))
    'full of woe'
    """
    verses = {
        0: "fair of face",
        1: "full of grace",
        2: "full of woe",
        3: "has far to go",
        4: "loving and giving",
        5: "works hard for a living",
        6: "bonny and blithe and good and gay",
    }
    return verses[d.weekday()]
