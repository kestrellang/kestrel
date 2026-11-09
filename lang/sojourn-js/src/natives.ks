module js

// The built-in functions.

import std.system.(Process)

@derive(Equatable)
enum NativeFn {
    // Global
    case Print
    case GlobalIsNaN
    case GlobalIsFinite
    case ParseInt
    case ParseFloat
    case Eval
    case EvalScript
    case GC
    case ThrowTypeError
    case ReturnThis
    // Object
    case ObjectCtor
    case ObjectKeys
    case ObjectValues
    case ObjectEntries
    case ObjectGetPrototypeOf
    case ObjectSetPrototypeOf
    case ObjectCreate
    case ObjectDefineProperty
    case ObjectDefineProperties
    case ObjectGetOwnPropertyDescriptor
    case ObjectGetOwnPropertyDescriptors
    case ObjectGetOwnPropertyNames
    case ObjectGetOwnPropertySymbols
    case ObjectFreeze
    case ObjectIsFrozen
    case ObjectSeal
    case ObjectIsSealed
    case ObjectPreventExtensions
    case ObjectIsExtensible
    case ObjectAssign
    case ObjectIs
    case ObjectFromEntries
    case ObjectHasOwn
    case HasOwnProperty
    case IsPrototypeOf
    case PropertyIsEnumerable
    case ObjectToString
    case ObjectToLocaleString
    case ObjectValueOf
    case ProtoGetter
    case ProtoSetter
    // Function
    case FunctionProto
    case FunctionCtor
    case FunctionCall
    case FunctionApply
    case FunctionBind
    case FunctionToString
    case FunctionHasInstance
    // Array
    case ArrayCtor
    case ArrayIsArray
    case ArrayOf
    case ArrayFrom
    case ArrayPush
    case ArrayPop
    case ArrayShift
    case ArrayUnshift
    case ArraySlice
    case ArraySplice
    case ArrayConcat
    case ArrayJoin
    case ArrayToLocaleString
    case ArrayToString
    case ArrayIndexOf
    case ArrayLastIndexOf
    case ArrayIncludes
    case ArrayForEach
    case ArrayMap
    case ArrayFilter
    case ArrayReduce
    case ArrayReduceRight
    case ArraySome
    case ArrayEvery
    case ArrayFind
    case ArrayFindIndex
    case ArrayFindLast
    case ArrayFindLastIndex
    case ArrayReverse
    case ArraySort
    case ArrayFill
    case ArrayKeys
    case ArrayValues
    case ArrayEntries
    case ArrayAt
    case ArrayFlat
    case ArrayIteratorNext
    // String
    case StringCtor
    case StringFromCharCode
    case StringFromCodePoint
    case StringCharAt
    case StringCharCodeAt
    case StringCodePointAt
    case StringIndexOf
    case StringLastIndexOf
    case StringIncludes
    case StringStartsWith
    case StringEndsWith
    case StringSlice
    case StringSubstring
    case StringSubstr
    case StringToUpperCase
    case StringToLowerCase
    case StringTrim
    case StringTrimStart
    case StringTrimEnd
    case StringSplit
    case StringRepeat
    case StringPadStart
    case StringPadEnd
    case StringConcat
    case StringToString
    case StringAt
    case StringLocaleCompare
    case StringIterator
    case StringIteratorNext
    // Number, Boolean, Symbol
    case NumberCtor
    case NumberIsNaN
    case NumberIsFinite
    case NumberIsInteger
    case NumberIsSafeInteger
    case NumberToString
    case NumberValueOf
    case NumberToFixed
    case BooleanCtor
    case BooleanToString
    case BooleanValueOf
    case SymbolCtor
    case SymbolFor
    case SymbolKeyFor
    case SymbolToString
    case SymbolValueOf
    case SymbolDescription
    case ErrorStackGet
    case ErrorStackSet
    // Errors
    case ErrorCtor
    case TypeErrorCtor
    case RangeErrorCtor
    case ReferenceErrorCtor
    case SyntaxErrorCtor
    case EvalErrorCtor
    case URIErrorCtor
    case ErrorToString
    case GeneratorFunctionCtor
    case AsyncFunctionCtor
    case AsyncGeneratorFunctionCtor
    // Set methods
    case SetUnion
    // BigInt
    case BigIntCtor
    case BigIntAsIntN
    case BigIntAsUintN
    case BigIntToString
    case BigIntToLocaleString
    case BigIntValueOf
    // Proxy
    case ProxyCtor
    case ProxyRevocable
    case ProxyRevoke
    // Typed arrays
    case ArrayBufferCtor
    case ArrayBufferIsView
    case ArrayBufferByteLength
    case ArrayBufferSlice
    case DetachArrayBuffer
    case TypedArrayCtor
    case TypedArrayKindCtor
    case TypedArrayFrom
    case TypedArrayOf
    case TABuffer
    case TAByteLength
    case TAByteOffset
    case TALength
    case TAToStringTag
    case TAAt
    case TACopyWithin
    case TAEntries
    case TAEvery
    case TAFill
    case TAFilter
    case TAFind
    case TAFindIndex
    case TAFindLast
    case TAFindLastIndex
    case TAForEach
    case TAIncludes
    case TAIndexOf
    case TAJoin
    case TAKeys
    case TALastIndexOf
    case TAMap
    case TAReduce
    case TAReduceRight
    case TAReverse
    case TASet
    case TASlice
    case TASome
    case TASort
    case TASubarray
    case TAToLocaleString
    case TAToReversed
    case TAToSorted
    case TAValues
    case TAWith
    case DataViewCtor
    case DVBuffer
    case DVByteLength
    case DVByteOffset
    case DVGet
    case DVSet
    case SetIntersection
    case SetDifference
    case SetSymmetricDifference
    case SetIsSubsetOf
    case SetIsSupersetOf
    case SetIsDisjointFrom
    // Iterator
    case IteratorCtor
    case IteratorFrom
    case IteratorCtorGet
    case IteratorCtorSet
    case IteratorTagGet
    case IteratorTagSet
    case IteratorMap
    case IteratorFilter
    case IteratorTake
    case IteratorDrop
    case IteratorFlatMap
    case IteratorReduce
    case IteratorToArray
    case IteratorForEach
    case IteratorSome
    case IteratorEvery
    case IteratorFind
    case IteratorHelperNext
    case IteratorHelperReturn
    case WrappedIteratorNext
    case WrappedIteratorReturn
    // Async iteration
    case AsyncGeneratorNext
    case AsyncGeneratorReturn
    case AsyncGeneratorThrow
    case AsyncGeneratorReturnSettled
    case AsyncFromSyncNext
    case AsyncFromSyncReturn
    case AsyncFromSyncThrow
    case AsyncFromSyncUnwrap
    case AsyncFromSyncCloseRejected
    // Promise
    case PromiseCtor
    case PromiseResolveFunction
    case PromiseRejectFunction
    case PromiseCapabilityExecutor
    case PromiseThen
    case PromiseCatch
    case PromiseFinally
    case PromiseThenFinally
    case PromiseCatchFinally
    case PromiseValueThunk
    case PromiseThrower
    case PromiseResolve
    case PromiseReject
    case PromiseWithResolvers
    case PromiseAll
    case PromiseAllSettled
    case PromiseAny
    case PromiseRace
    case PromiseAllResolveElement
    case PromiseAllSettledElement
    case PromiseAnyRejectElement
    case AsyncResume
    case AggregateErrorCtor
    // RegExp
    case RegExpCtor
    case RegExpExec
    case RegExpToString
    // Math
    case MathAbs
    case MathFloor
    case MathCeil
    case MathRound
    case MathTrunc
    case MathSign
    case MathSqrt
    case MathCbrt
    case MathPow
    case MathMin
    case MathMax
    case MathRandom
    case MathSin
    case MathCos
    case MathTan
    case MathAsin
    case MathAcos
    case MathAtan
    case MathAtan2
    case MathExp
    case MathLog
    case MathLog2
    case MathLog10
    case MathHypot
    // JSON, Reflect
    case JSONStringify
    case JSONParse
    case ReflectOwnKeys
    case ReflectGetPrototypeOf
    case ReflectSetPrototypeOf
    case ReflectHas
    case ReflectApply
    case ReflectConstruct
    case ReflectGet
    case ReflectSet
    case ReflectDefineProperty
    case ReflectDeleteProperty
    case ReflectGetOwnPropertyDescriptor
    case ReflectPreventExtensions
    case ReflectIsExtensible
    // natives_more.ks
    case ArrayCopyWithin
    case ArrayFlatMap
    case ArrayToReversed
    case ArrayToSorted
    case ArrayToSpliced
    case ArrayWith
    case DecodeURI
    case DecodeURIComponent
    case EncodeURI
    case EncodeURIComponent
    case Escape
    case MathAcosh
    case MathAsinh
    case MathAtanh
    case MathClz32
    case MathCosh
    case MathExpm1
    case MathFround
    case MathImul
    case MathLog1p
    case MathSinh
    case MathTanh
    case NumberToExponential
    case NumberToPrecision
    case ObjectDefineGetter
    case ObjectDefineSetter
    case ObjectGroupBy
    case ObjectLookupGetter
    case ObjectLookupSetter
    case StringAnchor
    case StringBig
    case StringBlink
    case StringBold
    case StringFixed
    case StringFontcolor
    case StringFontsize
    case StringIsWellFormed
    case StringItalics
    case StringLink
    case StringRaw
    case StringReplace
    case StringReplaceAll
    case StringSmall
    case StringStrike
    case StringSub
    case StringSup
    case StringToWellFormed
    case Unescape
    // natives_collections.ks
    case CollectionIteratorNext
    case MapClear
    case MapCtor
    case MapDelete
    case MapEntries
    case MapForEach
    case MapGet
    case MapGroupBy
    case MapHas
    case SetHas
    case SetDelete
    case SetClear
    case SetForEach
    case SetEntries
    case SetSize
    case WeakMapGet
    case WeakMapSet
    case WeakMapHas
    case WeakMapDelete
    case WeakSetAdd
    case WeakSetHas
    case WeakSetDelete
    case MapKeys
    case MapSet
    case MapSize
    case MapValues
    case SetAdd
    case SetCtor
    case SetValues
    case WeakMapCtor
    case WeakSetCtor
    // natives_date.ks
    case DateCtor
    case DateGetDate
    case DateGetDay
    case DateGetFullYear
    case DateGetHours
    case DateGetMilliseconds
    case DateGetMinutes
    case DateGetMonth
    case DateGetSeconds
    case DateGetTime
    case DateGetTimezoneOffset
    case DateGetYear
    case DateNow
    case DateParse
    case DateSetDate
    case DateSetFullYear
    case DateSetHours
    case DateSetMilliseconds
    case DateSetMinutes
    case DateSetMonth
    case DateSetSeconds
    case DateSetTime
    case DateSetYear
    case DateToDateString
    case DateToISOString
    case DateToJSON
    case DateToPrimitive
    case DateToString
    case DateToTimeString
    case DateToUTCString
    case DateUTC
    // natives_generator.ks
    case GeneratorNext
    case GeneratorReturn
    case GeneratorThrow
    // natives_regexp.ks
    case RegExpCompile
    case RegExpDotAll
    case RegExpFlags
    case RegExpGlobal
    case RegExpHasIndices
    case RegExpIgnoreCase
    case RegExpMultiline
    case RegExpSource
    case RegExpSticky
    case RegExpStringIteratorNext
    case RegExpSymbolMatch
    case RegExpSymbolMatchAll
    case RegExpSymbolReplace
    case RegExpSymbolSearch
    case RegExpSymbolSplit
    case RegExpTest
    case RegExpUnicode
    case RegExpUnicodeSets
    case StringMatch
    case StringMatchAll
    case StringSearch
}

func arg(args: Array[Value], i: Int64) -> Value {
    if i < args.count { args(i) } else { Value.undefined }
}

/// Clamps a relative index (negative counts from the end) into 0...len.
func relativeIndex(rel: Float64, len: Int64) -> Int64 {
    let l = Float64(from: len);
    if rel < 0.0 {
        let x = l + rel;
        return if x < 0.0 { 0 } else { x.toInt64() ?? 0 };
    }
    if rel > l { len } else { rel.toInt64() ?? 0 }
}

extend VM {
    /// Calls a built-in. Its receiver and arguments stay rooted while it
    /// runs, and any roots it leaves (say, on an early error return) are
    /// dropped after.
    mutating func callNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        let mark = self.roots.count;
        self.pushRoot(callee);
        self.pushRoot(this);
        self.pushRoot(newTarget);
        for a in args { self.pushRoot(a); }
        let result = self.callNativeUnrooted(n, callee, this, args, newTarget);
        self.truncateRoots(mark);
        result
    }

    mutating func callNativeUnrooted(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        match n {
            .Print => {
                var line = "";
                for i in 0..<args.count {
                    if i > 0 { line.append(" "); }
                    line.append(try self.toText(args(i)));
                }
                print("\(line)\n");
                return .Ok(Value.undefined);
            },
            .GlobalIsNaN => return .Ok(Value.bool((try self.toNumber(a0)).isNaN)),
            .GlobalIsFinite => return .Ok(Value.bool((try self.toNumber(a0)).isFinite)),
            .ParseInt => return self.parseIntNative(a0, a1),
            .ParseFloat => return self.parseFloatNative(a0),
            .Eval => return self.indirectEval(a0),
            .EvalScript => {
                let src = try self.toText(a0);
                return self.runScript(src, "evalScript");
            },
            .GC => {
                self.collectGarbage();
                return .Ok(Value.undefined);
            },
            .ThrowTypeError => return self.throwType("'caller', 'callee', and 'arguments' properties may not be accessed"),
            .ReturnThis => return .Ok(this),
            .FunctionProto => return .Ok(Value.undefined),
            .FunctionCtor => return self.functionConstructor(args),
            .GeneratorFunctionCtor => return self.dynamicFunction(args, "function*"),
            .AsyncFunctionCtor => return self.dynamicFunction(args, "async function"),
            .AsyncGeneratorFunctionCtor => return self.dynamicFunction(args, "async function*"),
            _ => {}
        }
        if let .Some(r) = try self.objectNative(n, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.functionNative(n, callee, this, args) { return .Ok(r); }
        if let .Some(r) = try self.arrayNative(n, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.stringNative(n, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.miscNative(n, callee, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.moreNative(n, this, args) { return .Ok(r); }
        if let .Some(r) = try self.collectionNative(n, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.dateNative(n, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.generatorNative(n, this, args) { return .Ok(r); }
        if let .Some(r) = try self.regexpNative(n, callee, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.promiseNative(n, callee, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.asyncIterationNative(n, callee, this, args) { return .Ok(r); }
        if let .Some(r) = try self.iteratorNative(n, callee, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.setMethodsNative(n, this, args) { return .Ok(r); }
        if let .Some(r) = try self.typedArrayNative(n, callee, this, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.proxyNative(n, callee, args, newTarget) { return .Ok(r); }
        if let .Some(r) = try self.bigintNative(n, this, args, newTarget) { return .Ok(r); }
        self.throwType("built-in not implemented")
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    mutating func toIntegerOrInfinity(v: Value) -> Result[Float64, Value] {
        let n = try self.toNumber(v);
        if n.isNaN { return .Ok(0.0); }
        if n.isInfinite { return .Ok(n); }
        .Ok(n.trunc() + 0.0)
    }

    /// ToLength(obj.length).
    mutating func lengthOf(o: Value) -> Result[Int64, Value] {
        if self.isArray(o) { return .Ok(self.heap.objects(o.asRef).length); }
        let l = try self.getV(o, self.names.length);
        let n = try self.toIntegerOrInfinity(l);
        if n <= 0.0 { return .Ok(0); }
        if n > 9007199254740991.0 { return .Ok(9007199254740991); }
        .Ok(n.toInt64() ?? 0)
    }

    mutating func getIndex(o: Value, i: Int64) -> Result[Value, Value] {
        if o.isObject {
            if let .ArrayObject = self.kindAt(o.asRef) {
                if i < self.heap.objects(o.asRef).elements.count {
                    let e = self.heap.objects(o.asRef).elements(i);
                    if not e.isEmpty { return .Ok(e); }
                }
            }
        }
        let k = self.internIndex(i);
        self.getV(o, k)
    }

    mutating func setIndex(o: Value, i: Int64, v: Value) -> Result[(), Value] {
        let k = self.internIndex(i);
        self.setV(o, k, v, true)
    }

    mutating func hasIndex(o: Value, i: Int64) -> Result[Bool, Value] {
        if let .ArrayObject = self.kindAt(o.asRef) {
            if i < self.heap.objects(o.asRef).elements.count {
                if not self.heap.objects(o.asRef).elements(i).isEmpty { return .Ok(true); }
            }
        }
        let k = self.internIndex(i);
        self.hasProperty(o, k)
    }

    mutating func setLength(o: Value, n: Int64) -> Result[(), Value] {
        self.setV(o, self.names.length, Value.int(n), true)
    }

    /// `this` as a string, for String.prototype methods.
    mutating func thisUnits(this: Value, name: String) -> Result[Units, Value] {
        if this.isNullish { return self.throwType("String.prototype.\(name) called on null or undefined"); }
        self.toUnits(this)
    }

    mutating func requireCallable(f: Value) -> Result[Value, Value] {
        if not self.isCallable(f) { return self.throwType("\(self.describe(f)) is not a function"); }
        .Ok(f)
    }

    // ------------------------------------------------------------------
    // Global functions
    // ------------------------------------------------------------------

    mutating func parseIntNative(s: Value, radixValue: Value) -> Result[Value, Value] {
        let u = unitsTrim(try self.toUnits(s), true, false);
        var i: Int64 = 0;
        var negative = false;
        if i < u.count and (u(i) == 45 or u(i) == 43) {
            negative = u(i) == 45;
            i = i + 1;
        }
        var radix = toInt32(try self.toNumber(radixValue));
        var stripPrefix = true;
        if radix != 0 {
            if radix < 2 or radix > 36 { return .Ok(Value.number(Float64.nan)); }
            if radix != 16 { stripPrefix = false; }
        } else {
            radix = 10;
        }
        if stripPrefix and i + 1 < u.count and u(i) == 48 and (u(i + 1) == 120 or u(i + 1) == 88) {
            i = i + 2;
            radix = 16;
        }
        var value = 0.0;
        var any = false;
        while i < u.count {
            let c = Int64(from: u(i));
            let d = if c >= 48 and c <= 57 { c - 48 } else if c >= 97 and c <= 122 { c - 87 } else if c >= 65 and c <= 90 { c - 55 } else { 99 };
            if d >= radix { break; }
            value = value * Float64(from: radix) + Float64(from: d);
            any = true;
            i = i + 1;
        }
        if not any { return .Ok(Value.number(Float64.nan)); }
        .Ok(Value.number(if negative { 0.0 - value } else { value }))
    }

    mutating func parseFloatNative(s: Value) -> Result[Value, Value] {
        let u = unitsTrim(try self.toUnits(s), true, false);
        // The longest prefix that's a decimal literal (or Infinity).
        let text = stringOf(u);
        var chars: [Char] = [];
        for c in text.chars { chars.append(c); }
        var start: Int64 = 0;
        var sign = "";
        if chars.count > 0 and (chars(0) == '+' or chars(0) == '-') {
            if chars(0) == '-' { sign = "-"; }
            start = 1;
        }
        var rest = "";
        for i in start..<chars.count { rest.append(char: chars(i)); }
        if rest.starts(with: "Infinity") {
            return .Ok(Value.number(if sign == "-" { 0.0 - Float64.infinity } else { Float64.infinity }));
        }
        var best = -1;
        var end = chars.count;
        while end > start {
            var sub: [Char] = [];
            for i in start..<end { sub.append(chars(i)); }
            if isDecimalLiteral(sub) and sub(0) != '+' and sub(0) != '-' {
                best = end;
                break;
            }
            end = end - 1;
        }
        if best < 0 { return .Ok(Value.number(Float64.nan)); }
        var lit = sign;
        for i in start..<best { lit.append(char: chars(i)); }
        .Ok(Value.number(Float64(parsing: lit) ?? Float64.nan))
    }
}
