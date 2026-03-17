switch (x) {
  case NaN:
    foo();
    break;

  case 2: {
    bar();
    break;
  }

  default: {
    baz();
    break;
  }
}
