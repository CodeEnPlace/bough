switch (x) {
  case 1:
    foo();
    break;

  case NaN: {
    bar();
    break;
  }

  default: {
    baz();
    break;
  }
}
