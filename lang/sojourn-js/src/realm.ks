module js

// Setting up the realm: the prototypes, constructors and global object,
// with each built-in function bound to a `NativeFn` case.

extend VM {
    /// A native function object.
    mutating func nativeFunction(n: NativeFn, name: String, length: Int64, isConstructor: Bool) -> Value {
        let f = self.newObjectKind(self.intrinsics.functionProto, .NativeFunction(n));
        let r = f.asRef;
        self.heap.objects(r).isConstructor = isConstructor;
        self.defineReadOnly(r, "length", Value.int(length));
        let nm = self.str(name);
        self.defineReadOnly(r, "name", nm);
        f
    }

    /// Installs a built-in method on `obj`.
    mutating func method(obj: Value, name: String, n: NativeFn, length: Int64) {
        let f = self.nativeFunction(n, name, length, false);
        let k = self.atom(name);
        self.defineHidden(obj.asRef, k, f);
    }

    mutating func symbolMethod(obj: Value, sym: Int64, name: String, n: NativeFn, length: Int64) {
        let f = self.nativeFunction(n, name, length, false);
        self.defineHidden(obj.asRef, symbolKey(sym), f);
    }

    mutating func getter(obj: Value, name: String, key: Key, n: NativeFn) {
        let f = self.nativeFunction(n, "get \(name)", 0, false);
        self.defineAccessorOwn(obj.asRef, key, f, Value.undefined, Attr.configurable);
    }

    /// A constructor `name` with prototype object `proto`, installed as a
    /// global.
    mutating func constructorFor(name: String, n: NativeFn, length: Int64, proto: Value) -> Value {
        let c = self.nativeFunction(n, name, length, true);
        self.defineOwn(c.asRef, self.names.prototype, proto, 0);
        self.defineHidden(proto.asRef, self.names.constructor, c);
        self.defineHidden(self.global.asRef, self.atom(name), c);
        c
    }

    mutating func constant(obj: Value, name: String, v: Value) {
        self.defineOwn(obj.asRef, self.atom(name), v, 0);
    }

    mutating func setupRealm() {
        // Well-known symbols take ids 0 through 7, in WellKnown's order.
        for d in ["Symbol.iterator", "Symbol.hasInstance", "Symbol.toPrimitive", "Symbol.toStringTag",
                  "Symbol.unscopables", "Symbol.asyncIterator", "Symbol.isConcatSpreadable", "Symbol.species"] {
            let s = self.str(d);
            let _ = self.newSymbol(s);
        }
        let objectProto = self.newObjectKind(Value.nullValue, .Ordinary);
        self.intrinsics.objectProto = objectProto;
        let functionProto = self.newObjectKind(objectProto, .NativeFunction(.FunctionProto));
        self.intrinsics.functionProto = functionProto;
        self.defineReadOnly(functionProto.asRef, "length", Value.int(0));
        let empty = self.str("");
        self.defineReadOnly(functionProto.asRef, "name", empty);
        self.global = self.newObjectKind(objectProto, .Ordinary);
        let g = self.global;
        self.defineHidden(g.asRef, self.atom("globalThis"), g);
        self.constant(g, "undefined", Value.undefined);
        self.constant(g, "NaN", Value.number(Float64.nan));
        self.constant(g, "Infinity", Value.number(Float64.infinity));

        self.intrinsics.throwTypeError = self.nativeFunction(.ThrowTypeError, "", 0, false);
        self.heap.objects(self.intrinsics.throwTypeError.asRef).extensible = false;

        // Object
        let objectCtor = self.constructorFor("Object", .ObjectCtor, 1, objectProto);
        for (name, n, len) in [
            ("keys", NativeFn.ObjectKeys, 1), ("values", NativeFn.ObjectValues, 1), ("entries", NativeFn.ObjectEntries, 1),
            ("getPrototypeOf", NativeFn.ObjectGetPrototypeOf, 1), ("setPrototypeOf", NativeFn.ObjectSetPrototypeOf, 2),
            ("create", NativeFn.ObjectCreate, 2), ("defineProperty", NativeFn.ObjectDefineProperty, 3),
            ("defineProperties", NativeFn.ObjectDefineProperties, 2),
            ("getOwnPropertyDescriptor", NativeFn.ObjectGetOwnPropertyDescriptor, 2),
            ("getOwnPropertyDescriptors", NativeFn.ObjectGetOwnPropertyDescriptors, 1),
            ("getOwnPropertyNames", NativeFn.ObjectGetOwnPropertyNames, 1),
            ("getOwnPropertySymbols", NativeFn.ObjectGetOwnPropertySymbols, 1),
            ("freeze", NativeFn.ObjectFreeze, 1), ("isFrozen", NativeFn.ObjectIsFrozen, 1),
            ("seal", NativeFn.ObjectSeal, 1), ("isSealed", NativeFn.ObjectIsSealed, 1),
            ("preventExtensions", NativeFn.ObjectPreventExtensions, 1), ("isExtensible", NativeFn.ObjectIsExtensible, 1),
            ("assign", NativeFn.ObjectAssign, 2), ("is", NativeFn.ObjectIs, 2), ("fromEntries", NativeFn.ObjectFromEntries, 1),
            ("hasOwn", NativeFn.ObjectHasOwn, 2)
        ] {
            self.method(objectCtor, name, n, len);
        }
        for (name, n, len) in [
            ("hasOwnProperty", NativeFn.HasOwnProperty, 1), ("isPrototypeOf", NativeFn.IsPrototypeOf, 1),
            ("propertyIsEnumerable", NativeFn.PropertyIsEnumerable, 1), ("toString", NativeFn.ObjectToString, 0),
            ("toLocaleString", NativeFn.ObjectToLocaleString, 0), ("valueOf", NativeFn.ObjectValueOf, 0)
        ] {
            self.method(objectProto, name, n, len);
        }
        let protoGet = self.nativeFunction(.ProtoGetter, "get __proto__", 0, false);
        let protoSet = self.nativeFunction(.ProtoSetter, "set __proto__", 1, false);
        self.defineAccessorOwn(objectProto.asRef, self.atom("__proto__"), protoGet, protoSet, Attr.configurable);

        // Function
        let functionCtor = self.constructorFor("Function", .FunctionCtor, 1, functionProto);
        let _ = functionCtor;
        self.method(functionProto, "call", .FunctionCall, 1);
        self.method(functionProto, "apply", .FunctionApply, 2);
        self.method(functionProto, "bind", .FunctionBind, 1);
        self.method(functionProto, "toString", .FunctionToString, 0);
        let hasInstance = self.nativeFunction(.FunctionHasInstance, "[Symbol.hasInstance]", 1, false);
        self.defineOwn(functionProto.asRef, symbolKey(WellKnown.hasInstance), hasInstance, 0);

        // Iterators
        let iteratorProto = self.newObject();
        self.intrinsics.iteratorProto = iteratorProto;
        self.symbolMethod(iteratorProto, WellKnown.iterator, "[Symbol.iterator]", .ReturnThis, 0);
        let arrayIteratorProto = self.newObjectKind(iteratorProto, .Ordinary);
        self.intrinsics.arrayIteratorProto = arrayIteratorProto;
        self.method(arrayIteratorProto, "next", .ArrayIteratorNext, 0);
        let tagA = self.str("Array Iterator");
        self.defineOwn(arrayIteratorProto.asRef, symbolKey(WellKnown.toStringTag), tagA, Attr.configurable);
        let stringIteratorProto = self.newObjectKind(iteratorProto, .Ordinary);
        self.intrinsics.stringIteratorProto = stringIteratorProto;
        self.method(stringIteratorProto, "next", .StringIteratorNext, 0);
        let tagS = self.str("String Iterator");
        self.defineOwn(stringIteratorProto.asRef, symbolKey(WellKnown.toStringTag), tagS, Attr.configurable);

        // Array
        let arrayProto = self.newObjectKind(objectProto, .ArrayObject);
        self.intrinsics.arrayProto = arrayProto;
        let arrayCtor = self.constructorFor("Array", .ArrayCtor, 1, arrayProto);
        self.method(arrayCtor, "isArray", .ArrayIsArray, 1);
        self.method(arrayCtor, "of", .ArrayOf, 0);
        self.method(arrayCtor, "from", .ArrayFrom, 1);
        for (name, n, len) in [
            ("push", NativeFn.ArrayPush, 1), ("pop", NativeFn.ArrayPop, 0), ("shift", NativeFn.ArrayShift, 0),
            ("unshift", NativeFn.ArrayUnshift, 1), ("slice", NativeFn.ArraySlice, 2), ("splice", NativeFn.ArraySplice, 2),
            ("concat", NativeFn.ArrayConcat, 1), ("join", NativeFn.ArrayJoin, 1), ("toString", NativeFn.ArrayToString, 0),
            ("indexOf", NativeFn.ArrayIndexOf, 1), ("lastIndexOf", NativeFn.ArrayLastIndexOf, 1),
            ("includes", NativeFn.ArrayIncludes, 1), ("forEach", NativeFn.ArrayForEach, 1), ("map", NativeFn.ArrayMap, 1),
            ("filter", NativeFn.ArrayFilter, 1), ("reduce", NativeFn.ArrayReduce, 1), ("reduceRight", NativeFn.ArrayReduceRight, 1),
            ("some", NativeFn.ArraySome, 1), ("every", NativeFn.ArrayEvery, 1), ("find", NativeFn.ArrayFind, 1),
            ("findIndex", NativeFn.ArrayFindIndex, 1), ("findLast", NativeFn.ArrayFindLast, 1),
            ("findLastIndex", NativeFn.ArrayFindLastIndex, 1), ("reverse", NativeFn.ArrayReverse, 0),
            ("sort", NativeFn.ArraySort, 1), ("fill", NativeFn.ArrayFill, 1), ("keys", NativeFn.ArrayKeys, 0),
            ("entries", NativeFn.ArrayEntries, 0), ("at", NativeFn.ArrayAt, 1), ("flat", NativeFn.ArrayFlat, 0)
        ] {
            self.method(arrayProto, name, n, len);
        }
        let values = self.nativeFunction(.ArrayValues, "values", 0, false);
        self.defineHidden(arrayProto.asRef, self.atom("values"), values);
        self.defineHidden(arrayProto.asRef, symbolKey(WellKnown.iterator), values);

        // String
        let emptyString = self.str("");
        let stringProto = self.newObjectKind(objectProto, .StringBox(emptyString));
        self.intrinsics.stringProto = stringProto;
        let stringCtor = self.constructorFor("String", .StringCtor, 1, stringProto);
        self.method(stringCtor, "fromCharCode", .StringFromCharCode, 1);
        self.method(stringCtor, "fromCodePoint", .StringFromCodePoint, 1);
        for (name, n, len) in [
            ("charAt", NativeFn.StringCharAt, 1), ("charCodeAt", NativeFn.StringCharCodeAt, 1),
            ("codePointAt", NativeFn.StringCodePointAt, 1), ("indexOf", NativeFn.StringIndexOf, 1),
            ("lastIndexOf", NativeFn.StringLastIndexOf, 1), ("includes", NativeFn.StringIncludes, 1),
            ("startsWith", NativeFn.StringStartsWith, 1), ("endsWith", NativeFn.StringEndsWith, 1),
            ("slice", NativeFn.StringSlice, 2), ("substring", NativeFn.StringSubstring, 2), ("substr", NativeFn.StringSubstr, 2),
            ("toUpperCase", NativeFn.StringToUpperCase, 0), ("toLowerCase", NativeFn.StringToLowerCase, 0),
            ("toLocaleUpperCase", NativeFn.StringToUpperCase, 0), ("toLocaleLowerCase", NativeFn.StringToLowerCase, 0),
            ("trim", NativeFn.StringTrim, 0), ("trimStart", NativeFn.StringTrimStart, 0), ("trimEnd", NativeFn.StringTrimEnd, 0),
            ("split", NativeFn.StringSplit, 2), ("repeat", NativeFn.StringRepeat, 1), ("padStart", NativeFn.StringPadStart, 1),
            ("padEnd", NativeFn.StringPadEnd, 1), ("concat", NativeFn.StringConcat, 1), ("toString", NativeFn.StringToString, 0),
            ("valueOf", NativeFn.StringToString, 0), ("at", NativeFn.StringAt, 1), ("localeCompare", NativeFn.StringLocaleCompare, 1)
        ] {
            self.method(stringProto, name, n, len);
        }
        self.symbolMethod(stringProto, WellKnown.iterator, "[Symbol.iterator]", .StringIterator, 0);

        // Number and Boolean
        let numberProto = self.newObjectKind(objectProto, .NumberBox(0.0));
        self.intrinsics.numberProto = numberProto;
        let numberCtor = self.constructorFor("Number", .NumberCtor, 1, numberProto);
        self.method(numberCtor, "isNaN", .NumberIsNaN, 1);
        self.method(numberCtor, "isFinite", .NumberIsFinite, 1);
        self.method(numberCtor, "isInteger", .NumberIsInteger, 1);
        self.method(numberCtor, "isSafeInteger", .NumberIsSafeInteger, 1);
        self.constant(numberCtor, "MAX_SAFE_INTEGER", Value.number(9007199254740991.0));
        self.constant(numberCtor, "MIN_SAFE_INTEGER", Value.number(-9007199254740991.0));
        self.constant(numberCtor, "MAX_VALUE", Value.number(Float64(bitPattern: 0x7FEFFFFFFFFFFFFF)));
        self.constant(numberCtor, "MIN_VALUE", Value.number(Float64(bitPattern: 1)));
        self.constant(numberCtor, "EPSILON", Value.number(Float64(bitPattern: 0x3CB0000000000000)));
        self.constant(numberCtor, "NaN", Value.number(Float64.nan));
        self.constant(numberCtor, "POSITIVE_INFINITY", Value.number(Float64.infinity));
        self.constant(numberCtor, "NEGATIVE_INFINITY", Value.number(0.0 - Float64.infinity));
        self.method(numberProto, "toString", .NumberToString, 1);
        self.method(numberProto, "toLocaleString", .NumberToString, 0);
        self.method(numberProto, "valueOf", .NumberValueOf, 0);
        self.method(numberProto, "toFixed", .NumberToFixed, 1);
        let booleanProto = self.newObjectKind(objectProto, .BooleanBox(false));
        self.intrinsics.booleanProto = booleanProto;
        let _ = self.constructorFor("Boolean", .BooleanCtor, 1, booleanProto);
        self.method(booleanProto, "toString", .BooleanToString, 0);
        self.method(booleanProto, "valueOf", .BooleanValueOf, 0);

        // Symbol
        let symbolProto = self.newObject();
        self.intrinsics.symbolProto = symbolProto;
        let symbolCtor = self.constructorFor("Symbol", .SymbolCtor, 0, symbolProto);
        self.heap.objects(symbolCtor.asRef).isConstructor = false;
        self.method(symbolCtor, "for", .SymbolFor, 1);
        self.method(symbolCtor, "keyFor", .SymbolKeyFor, 1);
        for (name, id) in [("iterator", WellKnown.iterator), ("hasInstance", WellKnown.hasInstance),
                           ("toPrimitive", WellKnown.toPrimitive), ("toStringTag", WellKnown.toStringTag),
                           ("unscopables", WellKnown.unscopables), ("asyncIterator", WellKnown.asyncIterator),
                           ("isConcatSpreadable", WellKnown.isConcatSpreadable), ("species", WellKnown.species)] {
            self.constant(symbolCtor, name, Value.symbol(id));
        }
        self.method(symbolProto, "toString", .SymbolToString, 0);
        self.method(symbolProto, "valueOf", .SymbolValueOf, 0);
        self.getter(symbolProto, "description", self.atom("description"), .SymbolDescription);
        let tagSym = self.str("Symbol");
        self.defineOwn(symbolProto.asRef, symbolKey(WellKnown.toStringTag), tagSym, Attr.configurable);

        // Errors
        let errorProto = self.newObject();
        self.intrinsics.errorProto = errorProto;
        let errorCtor = self.constructorFor("Error", .ErrorCtor, 1, errorProto);
        self.method(errorProto, "toString", .ErrorToString, 0);
        let errName = self.str("Error");
        self.defineHidden(errorProto.asRef, self.names.name, errName);
        self.defineHidden(errorProto.asRef, self.names.message, emptyString);
        self.intrinsics.typeErrorProto = self.makeErrorType("TypeError", .TypeErrorCtor, errorCtor, errorProto);
        self.intrinsics.rangeErrorProto = self.makeErrorType("RangeError", .RangeErrorCtor, errorCtor, errorProto);
        self.intrinsics.referenceErrorProto = self.makeErrorType("ReferenceError", .ReferenceErrorCtor, errorCtor, errorProto);
        self.intrinsics.syntaxErrorProto = self.makeErrorType("SyntaxError", .SyntaxErrorCtor, errorCtor, errorProto);
        self.intrinsics.evalErrorProto = self.makeErrorType("EvalError", .EvalErrorCtor, errorCtor, errorProto);
        self.intrinsics.uriErrorProto = self.makeErrorType("URIError", .URIErrorCtor, errorCtor, errorProto);

        // RegExp (objects only; matching isn't implemented yet)
        let regexpProto = self.newObject();
        self.intrinsics.regexpProto = regexpProto;
        let _ = self.constructorFor("RegExp", .RegExpCtor, 2, regexpProto);
        self.method(regexpProto, "exec", .RegExpExec, 1);
        self.method(regexpProto, "test", .RegExpExec, 1);
        self.method(regexpProto, "toString", .RegExpToString, 0);

        // Math
        let math = self.newObject();
        self.defineHidden(g.asRef, self.atom("Math"), math);
        let tagM = self.str("Math");
        self.defineOwn(math.asRef, symbolKey(WellKnown.toStringTag), tagM, Attr.configurable);
        for (name, n, len) in [
            ("abs", NativeFn.MathAbs, 1), ("floor", NativeFn.MathFloor, 1), ("ceil", NativeFn.MathCeil, 1),
            ("round", NativeFn.MathRound, 1), ("trunc", NativeFn.MathTrunc, 1), ("sign", NativeFn.MathSign, 1),
            ("sqrt", NativeFn.MathSqrt, 1), ("cbrt", NativeFn.MathCbrt, 1), ("pow", NativeFn.MathPow, 2),
            ("min", NativeFn.MathMin, 2), ("max", NativeFn.MathMax, 2), ("random", NativeFn.MathRandom, 0),
            ("sin", NativeFn.MathSin, 1), ("cos", NativeFn.MathCos, 1), ("tan", NativeFn.MathTan, 1),
            ("asin", NativeFn.MathAsin, 1), ("acos", NativeFn.MathAcos, 1), ("atan", NativeFn.MathAtan, 1),
            ("atan2", NativeFn.MathAtan2, 2), ("exp", NativeFn.MathExp, 1), ("log", NativeFn.MathLog, 1),
            ("log2", NativeFn.MathLog2, 1), ("log10", NativeFn.MathLog10, 1), ("hypot", NativeFn.MathHypot, 2)
        ] {
            self.method(math, name, n, len);
        }
        self.constant(math, "PI", Value.number(3.141592653589793));
        self.constant(math, "E", Value.number(2.718281828459045));
        self.constant(math, "LN2", Value.number(0.6931471805599453));
        self.constant(math, "LN10", Value.number(2.302585092994046));
        self.constant(math, "LOG2E", Value.number(1.4426950408889634));
        self.constant(math, "LOG10E", Value.number(0.4342944819032518));
        self.constant(math, "SQRT2", Value.number(1.4142135623730951));
        self.constant(math, "SQRT1_2", Value.number(0.7071067811865476));

        // JSON and Reflect
        let json = self.newObject();
        self.defineHidden(g.asRef, self.atom("JSON"), json);
        self.method(json, "stringify", .JSONStringify, 3);
        let reflect = self.newObject();
        self.defineHidden(g.asRef, self.atom("Reflect"), reflect);
        for (name, n, len) in [
            ("ownKeys", NativeFn.ReflectOwnKeys, 1), ("getPrototypeOf", NativeFn.ReflectGetPrototypeOf, 1),
            ("setPrototypeOf", NativeFn.ReflectSetPrototypeOf, 2), ("has", NativeFn.ReflectHas, 2),
            ("apply", NativeFn.ReflectApply, 3), ("construct", NativeFn.ReflectConstruct, 2),
            ("get", NativeFn.ReflectGet, 2), ("set", NativeFn.ReflectSet, 3),
            ("defineProperty", NativeFn.ReflectDefineProperty, 3), ("deleteProperty", NativeFn.ReflectDeleteProperty, 2),
            ("getOwnPropertyDescriptor", NativeFn.ReflectGetOwnPropertyDescriptor, 2),
            ("isExtensible", NativeFn.ObjectIsExtensible, 1), ("preventExtensions", NativeFn.ReflectPreventExtensions, 1)
        ] {
            self.method(reflect, name, n, len);
        }

        // Global functions
        for (name, n, len) in [
            ("print", NativeFn.Print, 1), ("isNaN", NativeFn.GlobalIsNaN, 1), ("isFinite", NativeFn.GlobalIsFinite, 1),
            ("parseInt", NativeFn.ParseInt, 2), ("parseFloat", NativeFn.ParseFloat, 1), ("eval", NativeFn.Eval, 1)
        ] {
            self.method(g, name, n, len);
        }
        let parseIntFn = self.ownValue(g.asRef, self.atom("parseInt"));
        let parseFloatFn = self.ownValue(g.asRef, self.atom("parseFloat"));
        self.defineHidden(numberCtor.asRef, self.atom("parseInt"), parseIntFn);
        self.defineHidden(numberCtor.asRef, self.atom("parseFloat"), parseFloatFn);

        // $262, the test262 host hooks.
        let host = self.newObject();
        self.defineHidden(g.asRef, self.atom("$262"), host);
        self.defineHidden(host.asRef, self.atom("global"), g);
        self.method(host, "evalScript", .EvalScript, 1);
        self.method(host, "gc", .GC, 0);
    }

    mutating func makeErrorType(name: String, n: NativeFn, errorCtor: Value, errorProto: Value) -> Value {
        let proto = self.newObjectKind(errorProto, .Ordinary);
        let c = self.constructorFor(name, n, 1, proto);
        self.heap.objects(c.asRef).proto = errorCtor;
        let nm = self.str(name);
        self.defineHidden(proto.asRef, self.names.name, nm);
        let empty = self.str("");
        self.defineHidden(proto.asRef, self.names.message, empty);
        proto
    }
}
