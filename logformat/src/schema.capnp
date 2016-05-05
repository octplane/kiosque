@0xdbfc80a51e228642;

struct Map(Key, Value) {
  entries @0 :List(Entry);
  struct Entry {
    key @0 :Key;
    value @1 :Value;
  }
}

struct Logline {
  # This is time since EPOCH in µs (1.10e-6 s)
  time @0 :UInt64;
  facility @1 :Text;
  hostname @2 :Text;
  facets @3 :Map(Text, Text);
}

struct Logblock {
  startTime @0 :UInt64;
  endTime @1: UInt64;
  entries @2: List(Logline);
}


