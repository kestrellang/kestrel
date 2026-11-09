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
        self.hiddenConstructor("GeneratorFunction", .GeneratorFunctionCtor, fnProto);
    }

    /// A constructor reachable only as `proto.constructor`, like
    /// %GeneratorFunction%: inherits from Function.
    mutating func hiddenConstructor(name: String, n: NativeFn, proto: Value) {
        let c = self.nativeFunction(n, name, 1, true);
        let functionCtor = self.ownValue(self.global.asRef, self.atom("Function"));
        self.heap.objects(c.asRef).proto = functionCtor;
        self.defineOwn(c.asRef, self.names.prototype, proto, 0);
        self.defineOwn(proto.asRef, self.names.constructor, c, Attr.configurable);
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
            if let .Generator(id) = self.kindAt(this.asRef) {
                isGenerator = not self.heap.generators(id).isAsyncGenerator and self.heap.generators(id).asyncPromise.isUndefined;
            }
        }
        if not isGenerator { return self.throwType("next method called on incompatible receiver"); }
        let r = try self.resumeGenerator(this, arg(args, 0), mode);
        .Ok(.Some(r))
    }
}
