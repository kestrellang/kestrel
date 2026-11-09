module js

// %GeneratorFunction.prototype% and %GeneratorPrototype% (ECMA-262 §27.3-27.5).

extend VM {
    mutating func setupGenerators() {
        let fnProto = self.newObjectKind(self.intrinsics.functionProto, .Ordinary);
        let genProto = self.newObjectKind(self.intrinsics.iteratorProto, .Ordinary);
        self.intrinsics.generatorFunctionProto = fnProto;
        self.intrinsics.generatorProto = genProto;
        self.defineOwn(fnProto.asRef, self.names.prototype, genProto, Attr.configurable);
        self.defineOwn(genProto.asRef, self.names.constructor, fnProto, Attr.configurable);
        let fnTag = self.str("GeneratorFunction");
        self.defineOwn(fnProto.asRef, symbolKey(WellKnown.toStringTag), fnTag, Attr.configurable);
        let genTag = self.str("Generator");
        self.defineOwn(genProto.asRef, symbolKey(WellKnown.toStringTag), genTag, Attr.configurable);
        self.method(genProto, "next", .GeneratorNext, 1);
        self.method(genProto, "return", .GeneratorReturn, 1);
        self.method(genProto, "throw", .GeneratorThrow, 1);
    }

    mutating func generatorNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let mode = match n {
            .GeneratorNext => 0,
            .GeneratorThrow => 1,
            .GeneratorReturn => 2,
            _ => return .Ok(.None)
        };
        var isGenerator = false;
        if this.isObject {
            if let .Generator(_) = self.kindAt(this.asRef) { isGenerator = true; }
        }
        if not isGenerator { return self.throwType("next method called on incompatible receiver"); }
        let r = try self.resumeGenerator(this, arg(args, 0), mode);
        .Ok(.Some(r))
    }
}
