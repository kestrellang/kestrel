1. Summary

| Error Message | Count |
| :--- | :--- |
| if condition must conform to `BooleanConditional`, found `_` | 18 |
| cannot access member on type '[UInt8]' | 12 |
| no method 'write' on type 'H' matches the provided arguments | 11 |
| no matching initializer for struct 'UInt32' | 10 |
| no matching initializer for struct 'UInt8' | 10 |
| type arguments cannot be applied to a language intrinsic | 9 |
| member not found: `equals` on type `<error>` | 8 |
| cannot find type 'Int' in this scope | 6 |
| member 'substringBytes' is private and not accessible from this scope | 6 |
| member not found: `shiftLeft` on type `<error>` | 6 |
| undefined name 'Buffer' | 6 |
| cannot find type 'Array' in this scope | 4 |
| cannot use 'self' in free function | 4 |
| type mismatch: expected `U`, found `U` | 4 |
| undefined name 'ArcBox' | 4 |
| cannot find type 'Buffer' in this scope | 3 |
| no method 'byteAt' on type 'String[A]' matches the provided arguments | 3 |
| undefined name 'Dictionary' | 3 |
| cannot access member on type 'I16' | 2 |
| cannot access member on type 'I32' | 2 |
| cannot access member on type 'I64' | 2 |
| cannot assign to this expression | 2 |
| cannot call 'description' on type 'E' | 2 |
| cannot find type 'DictionaryIterator' in this scope | 2 |
| cannot find type 'I' in this scope | 2 |
| could not infer type for 1 placeholder(s) | 2 |
| member 'raw' is private and not accessible from this scope | 2 |
| member not found: `lessThan` on type `<error>` | 2 |
| no matching overload for 'decodeUtf8' with 2 argument(s) | 2 |
| no method 'ensureCapacity' on type 'String[A]' matches the provided arguments | 2 |
| non-exhaustive match expression | 2 |
| symbol 'Array' not found in module 'std.collections' | 2 |
| 'Equal' is not a type | 1 |
| 'Optional' is not a protocol; bound must be a protocol | 1 |
| Parse error: found 'RBrace' at 3675..3676 expected something else, 'LParen', 'Dot', 'Bang', 'LBrace', 'Equals', or 'Semicolon' | 1 |
| Parse error: found 'Underscore' at 2353..2354 expected something else, 'Mutating', 'Consuming', or 'RParen' | 1 |
| Parse error: found 'Underscore' at 6127..6128 expected something else, 'Mutating', 'Consuming', or 'RParen' | 1 |
| cannot access member on type 'I.Item' | 1 |
| cannot access member on type 'Inner.Iter' | 1 |
| cannot access member on type 'Int' | 1 |
| cannot assign to immutable field 'pointee' | 1 |
| cannot find type 'ArcBox' in this scope | 1 |
| cannot find type 'Dictionary' in this scope | 1 |
| initializer does not initialize all fields: 'storage' | 1 |
| member 'storage' is private and not accessible from this scope | 1 |
| member not found: `add` on type `String` | 1 |
| member not found: `logicalNot` on type `<error>` | 1 |
| member not found: `multiply` on type `<error>` | 1 |
| no matching initializer for struct 'UInt64' | 1 |
| no member 'toBytes' on type 'UInt64' | 1 |
| no method 'action' on type 'InspectIterator[I]' | 1 |
| no method 'insert' on type 'Set[T, A]' matches the provided arguments | 1 |
| no method 'predicate' on type 'FilterIterator[I]' | 1 |
| no method 'predicate' on type 'SkipWhileIterator[I]' | 1 |
| no method 'predicate' on type 'TakeWhileIterator[I]' | 1 |
| no method 'transform' on type 'FilterMapIterator[I, U]' | 1 |
| no method 'transform' on type 'FlatMapIterator[I, Inner]' | 1 |
| struct `SplitIterator` has Cloneable field `string` but does not conform to Cloneable | 1 |
| symbol 'ArcBox' not found in module 'std.memory' | 1 |
| symbol 'Buffer' not found in module 'std.memory' | 1 |
| symbol 'Dictionary' not found in module 'std.collections' | 1 |
| symbol 'DictionaryIterator' not found in module 'std.collections' | 1 |
| symbol 'Nil' not found in module 'std.ops' | 1 |
| type 'Break' is ambiguous | 1 |
| type 'BytesIterator' does not satisfy bound | 1 |
| type 'CharsIterator' does not satisfy bound | 1 |
| type 'ClosedRangeIterator' does not satisfy bound | 1 |
| type 'CodePointsIterator' does not satisfy bound | 1 |
| type 'Continue' is ambiguous | 1 |
| type 'LinesIterator' does not satisfy bound | 1 |
| type 'RangeIterator' does not satisfy bound | 1 |
| type 'SelfType' does not accept type arguments | 1 |
| type 'SetIterator' does not satisfy bound | 1 |
| type 'Slice' is ambiguous | 1 |
| type 'SliceIterator' does not satisfy bound | 1 |
| type mismatch: expected `(T) -> Optional[U]`, found `(T) -> Optional[U]` | 1 |
| type mismatch: expected `(T) -> Result[U, E]`, found `(T) -> Result[U, E]` | 1 |
| type mismatch: expected `Optional[I.Item]`, found `Optional[Optional[I.Item]]` | 1 |
| type mismatch: expected `Optional[U]`, found `Optional[U]` | 1 |
| type mismatch: expected `Result[U, E]`, found `Result[U, E]` | 1 |
| undefined name 'bitwiseAnd' | 1 |
| undefined name 'bitwiseNot' | 1 |
| while condition must conform to `BooleanConditional`, found `_` | 1 |

2. Details

if condition must conform to `BooleanConditional`, found `_`
Call Sites
- lang/std/iter/adapters.ks:216:16
- lang/std/memory/allocator.ks:78:12
- lang/std/text/views.ks:222:16
- lang/std/text/views.ks:226:23
- lang/std/text/string.ks:104:12
- lang/std/text/string.ks:111:12
- lang/std/text/string.ks:112:34
- lang/std/text/string.ks:223:16
- lang/std/text/string.ks:236:16
- lang/std/text/char.ks:179:8
- lang/std/text/char.ks:182:15
- lang/std/text/char.ks:185:15
- lang/std/text/char.ks:189:12
- lang/std/text/char.ks:192:15
- lang/std/text/char.ks:197:12
- lang/std/text/char.ks:202:15
- lang/std/text/char.ks:208:12
- lang/std/collections/set.ks:57:12

cannot access member on type '[UInt8]'
Call Sites
- lang/std/text/string.ks:73:25
- lang/std/text/string.ks:137:59
- lang/std/text/char.ks:89:17
- lang/std/text/char.ks:92:17
- lang/std/text/char.ks:93:17
- lang/std/text/char.ks:96:17
- lang/std/text/char.ks:97:17
- lang/std/text/char.ks:98:17
- lang/std/text/char.ks:101:17
- lang/std/text/char.ks:102:17
- lang/std/text/char.ks:103:17
- lang/std/text/char.ks:104:17

no method 'write' on type 'H' matches the provided arguments
Call Sites
- lang/std/core/int32.ks:63:9
- lang/std/core/int16.ks:62:9
- lang/std/core/uint64.ks:61:9
- lang/std/core/int8.ks:62:9
- lang/std/core/uint8.ks:61:9
- lang/std/core/uint32.ks:61:9
- lang/std/core/uint16.ks:61:9
- lang/std/core/int64.ks:63:9
- lang/std/core/bool.ks:33:13
- lang/std/core/bool.ks:35:13
- lang/std/collections/set.ks:229:9

no matching initializer for struct 'UInt32'
Call Sites
- lang/std/text/char.ks:181:40
- lang/std/text/char.ks:190:22
- lang/std/text/char.ks:190:51
- lang/std/text/char.ks:198:22
- lang/std/text/char.ks:199:22
- lang/std/text/char.ks:200:21
- lang/std/text/char.ks:211:22
- lang/std/text/char.ks:212:22
- lang/std/text/char.ks:213:22
- lang/std/text/char.ks:214:21

no matching initializer for struct 'UInt8'
Call Sites
- lang/std/text/char.ks:89:31
- lang/std/text/char.ks:92:31
- lang/std/text/char.ks:93:31
- lang/std/text/char.ks:96:31
- lang/std/text/char.ks:97:31
- lang/std/text/char.ks:98:31
- lang/std/text/char.ks:101:31
- lang/std/text/char.ks:102:31
- lang/std/text/char.ks:103:31
- lang/std/text/char.ks:104:31

type arguments cannot be applied to a language intrinsic
Call Sites
- lang/std/memory/pointer.ks:36:22
- lang/std/memory/pointer.ks:61:22
- lang/std/memory/pointer.ks:70:29
- lang/std/memory/pointer.ks:74:26
- lang/std/memory/pointer.ks:86:69
- lang/std/memory/pointer.ks:86:22
- lang/std/memory/pointer.ks:90:25
- lang/std/memory/layout.ks:18:22
- lang/std/memory/layout.ks:18:51

member not found: `equals` on type `<error>`
Call Sites
- lang/std/result/error.ks:24:45
- lang/std/result/error.ks:25:39
- lang/std/text/views.ks:222:16
- lang/std/text/views.ks:226:23
- lang/std/text/views.ks:230:63
- lang/std/text/string.ks:112:34
- lang/std/text/char.ks:146:9
- lang/std/text/char.ks:160:9

cannot find type 'Int' in this scope
Call Sites
- lang/std/ffi/libc.ks:9:26
- lang/std/ffi/libc.ks:15:51
- lang/std/ffi/libc.ks:19:72
- lang/std/ffi/libc.ks:22:73
- lang/std/ffi/libc.ks:25:48
- lang/std/ffi/libc.ks:25:56

member 'substringBytes' is private and not accessible from this scope
Call Sites
- lang/std/text/views.ks:223:40
- lang/std/text/views.ks:227:40
- lang/std/text/views.ks:241:38
- lang/std/text/string.ks:382:42
- lang/std/text/string.ks:400:42
- lang/std/text/string.ks:410:38

member not found: `shiftLeft` on type `<error>`
Call Sites
- lang/std/text/char.ks:190:22
- lang/std/text/char.ks:198:22
- lang/std/text/char.ks:199:22
- lang/std/text/char.ks:211:22
- lang/std/text/char.ks:212:22
- lang/std/text/char.ks:213:22

undefined name 'Buffer'
Call Sites
- lang/std/memory/allocator.ks:70:23
- lang/std/memory/allocator.ks:120:23
- lang/std/text/string.ks:32:21
- lang/std/text/string.ks:39:21
- lang/std/text/string.ks:46:21
- lang/std/text/string.ks:59:21

cannot find type 'Array' in this scope
Call Sites
- lang/std/text/string.ks:68:29
- lang/std/text/char.ks:129:30
- lang/std/text/char.ks:135:29
- lang/std/text/char.ks:139:28

cannot use 'self' in free function
Call Sites
- lang/std/result/result.ks:28:15
- lang/std/result/result.ks:35:15
- lang/std/result/optional.ks:30:15
- lang/std/result/optional.ks:37:15

type mismatch: expected `U`, found `U`
Call Sites
- lang/std/result/result.ks:152:9
- lang/std/result/result.ks:152:22
- lang/std/result/optional.ks:126:9
- lang/std/result/optional.ks:126:22

undefined name 'ArcBox'
Call Sites
- lang/std/text/string.ks:31:24
- lang/std/text/string.ks:38:24
- lang/std/text/string.ks:45:24
- lang/std/text/string.ks:58:24

cannot find type 'Buffer' in this scope
Call Sites
- lang/std/memory/allocator.ks:66:25
- lang/std/memory/allocator.ks:111:25
- lang/std/text/string.ks:25:21

no method 'byteAt' on type 'String[A]' matches the provided arguments
Call Sites
- lang/std/text/views.ks:77:24
- lang/std/text/views.ks:221:24
- lang/std/text/views.ks:230:63

undefined name 'Dictionary'
Call Sites
- lang/std/collections/set.ks:27:21
- lang/std/collections/set.ks:31:21
- lang/std/collections/set.ks:35:21

cannot access member on type 'I16'
Call Sites
- lang/std/core/int16.ks:62:29
- lang/std/core/uint16.ks:61:29

cannot access member on type 'I32'
Call Sites
- lang/std/core/int32.ks:63:29
- lang/std/core/uint32.ks:61:29

cannot access member on type 'I64'
Call Sites
- lang/std/core/uint64.ks:61:29
- lang/std/core/int64.ks:63:29

cannot assign to this expression
Call Sites
- lang/std/text/string.ks:146:9
- lang/std/text/string.ks:345:9

cannot call 'description' on type 'E'
Call Sites
- lang/std/result/result.ks:58:68
- lang/std/result/result.ks:87:56

cannot find type 'DictionaryIterator' in this scope
Call Sites
- lang/std/collections/set.ks:237:27
- lang/std/collections/set.ks:239:27

cannot find type 'I' in this scope
Call Sites
- lang/std/iter/iterator.ks:22:24
- lang/std/collections/set.ks:39:31

could not infer type for 1 placeholder(s)
Call Sites
- lang/std/iter/adapters.ks:92:20
- lang/std/text/string.ks:381:20

member 'raw' is private and not accessible from this scope
Call Sites
- lang/std/memory/allocator.ks:48:18
- lang/std/memory/allocator.ks:52:34

member not found: `lessThan` on type `<error>`
Call Sites
- lang/std/text/string.ks:111:12
- lang/std/text/string.ks:116:23

no matching overload for 'decodeUtf8' with 2 argument(s)
Call Sites
- lang/std/text/views.ks:127:28
- lang/std/text/string.ks:381:32

no method 'ensureCapacity' on type 'String[A]' matches the provided arguments
Call Sites
- lang/std/text/string.ks:127:9
- lang/std/text/string.ks:137:9

non-exhaustive match expression
Call Sites
- lang/std/result/result.ks:171:9
- lang/std/text/char.ks:87:9

symbol 'Array' not found in module 'std.collections'
Call Sites
- lang/std/text/string.ks:8:25
- lang/std/text/char.ks:7:25

'Equal' is not a type
Call Sites
- lang/std/core/ordering.ks:8:34

'Optional' is not a protocol; bound must be a protocol
Call Sites
- lang/std/result/result.ks:170:64

Parse error: found 'RBrace' at 3675..3676 expected something else, 'LParen', 'Dot', 'Bang', 'LBrace', 'Equals', or 'Semicolon'
Call Sites
- lang/std/memory/buffer.ks:113:5

Parse error: found 'Underscore' at 2353..2354 expected something else, 'Mutating', 'Consuming', or 'RParen'
Call Sites
- lang/std/collections/array.ks:96:41

Parse error: found 'Underscore' at 6127..6128 expected something else, 'Mutating', 'Consuming', or 'RParen'
Call Sites
- lang/std/collections/dictionary.ks:221:33

cannot access member on type 'I.Item'
Call Sites
- lang/std/iter/adapters.ks:362:26

cannot access member on type 'Inner.Iter'
Call Sites
- lang/std/iter/adapters.ks:86:38

cannot access member on type 'Int'
Call Sites
- lang/std/text/views.ks:261:9

cannot assign to immutable field 'pointee'
Call Sites
- lang/std/memory/allocator.ks:149:9

cannot find type 'ArcBox' in this scope
Call Sites
- lang/std/text/string.ks:22:26

cannot find type 'Dictionary' in this scope
Call Sites
- lang/std/collections/set.ks:23:23

initializer does not initialize all fields: 'storage'
Call Sites
- lang/std/text/string.ks:52:5

member 'storage' is private and not accessible from this scope
Call Sites
- lang/std/text/views.ks:56:36

member not found: `add` on type `String`
Call Sites
- lang/std/result/result.ks:58:39

member not found: `logicalNot` on type `<error>`
Call Sites
- lang/std/iter/adapters.ks:216:16

member not found: `multiply` on type `<error>`
Call Sites
- lang/std/text/string.ks:117:27

no matching initializer for struct 'UInt64'
Call Sites
- lang/std/core/protocols.ks:110:37

no member 'toBytes' on type 'UInt64'
Call Sites
- lang/std/collections/set.ks:229:42

no method 'action' on type 'InspectIterator[I]'
Call Sites
- lang/std/iter/adapters.ks:116:13

no method 'insert' on type 'Set[T, A]' matches the provided arguments
Call Sites
- lang/std/collections/set.ks:42:13

no method 'predicate' on type 'FilterIterator[I]'
Call Sites
- lang/std/iter/adapters.ks:39:16

no method 'predicate' on type 'SkipWhileIterator[I]'
Call Sites
- lang/std/iter/adapters.ks:216:20

no method 'predicate' on type 'TakeWhileIterator[I]'
Call Sites
- lang/std/iter/adapters.ks:164:16

no method 'transform' on type 'FilterMapIterator[I, U]'
Call Sites
- lang/std/iter/adapters.ks:61:29

no method 'transform' on type 'FlatMapIterator[I, Inner]'
Call Sites
- lang/std/iter/adapters.ks:93:38

struct `SplitIterator` has Cloneable field `string` but does not conform to Cloneable
Call Sites
- lang/std/text/string.ks:359:5

symbol 'ArcBox' not found in module 'std.memory'
Call Sites
- lang/std/text/string.ks:7:31

symbol 'Buffer' not found in module 'std.memory'
Call Sites
- lang/std/text/string.ks:7:39

symbol 'Dictionary' not found in module 'std.collections'
Call Sites
- lang/std/collections/set.ks:9:25

symbol 'DictionaryIterator' not found in module 'std.collections'
Call Sites
- lang/std/collections/set.ks:9:37

symbol 'Nil' not found in module 'std.ops'
Call Sites
- lang/std/result/optional.ks:7:42

type 'Break' is ambiguous
Call Sites
- lang/std/result/error.ks:16:16

type 'BytesIterator' does not satisfy bound
Call Sites
- lang/std/text/views.ks:13:5

type 'CharsIterator' does not satisfy bound
Call Sites
- lang/std/text/views.ks:143:5

type 'ClosedRangeIterator' does not satisfy bound
Call Sites
- lang/std/ops/range.ks:104:5

type 'CodePointsIterator' does not satisfy bound
Call Sites
- lang/std/text/views.ks:89:5

type 'Continue' is ambiguous
Call Sites
- lang/std/result/error.ks:15:19

type 'LinesIterator' does not satisfy bound
Call Sites
- lang/std/text/views.ks:186:5

type 'RangeIterator' does not satisfy bound
Call Sites
- lang/std/ops/range.ks:50:5

type 'SelfType' does not accept type arguments
Call Sites
- lang/std/iter/iterator.ks:28:45

type 'SetIterator' does not satisfy bound
Call Sites
- lang/std/collections/set.ks:20:5

type 'Slice' is ambiguous
Call Sites
- lang/std/text/char.ks:172:31

type 'SliceIterator' does not satisfy bound
Call Sites
- lang/std/memory/pointer.ks:104:5

type mismatch: expected `(T) -> Optional[U]`, found `(T) -> Optional[U]`
Call Sites
- lang/std/result/optional.ks:126:22

type mismatch: expected `(T) -> Result[U, E]`, found `(T) -> Result[U, E]`
Call Sites
- lang/std/result/result.ks:152:22

type mismatch: expected `Optional[I.Item]`, found `Optional[Optional[I.Item]]`
Call Sites
- lang/std/iter/adapters.ks:394:20

type mismatch: expected `Optional[U]`, found `Optional[U]`
Call Sites
- lang/std/result/optional.ks:126:9

type mismatch: expected `Result[U, E]`, found `Result[U, E]`
Call Sites
- lang/std/result/result.ks:152:9

undefined name 'bitwiseAnd'
Call Sites
- lang/std/memory/allocator.ks:76:66

undefined name 'bitwiseNot'
Call Sites
- lang/std/memory/allocator.ks:76:100

while condition must conform to `BooleanConditional`, found `_`
Call Sites
- lang/std/text/string.ks:116:23
