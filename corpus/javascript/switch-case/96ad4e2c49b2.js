switch (x) {
  case 1:
    foo();
    break;

  case -Infinity: {
    bar();
    break;
  }

  default: {
    baz();
    break;
  }
}
