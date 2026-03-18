class C:
    def __enter__(self):
        return self
    def __exit__(self, *a):
        pass

with C() as c:
    pass
