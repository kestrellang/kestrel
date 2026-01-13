1. Summary

| Error Message | Count |
| :--- | :--- |
| undefined name 'lang' | 51 |
| type `Self` does not conform to protocol `ExpressibleByIntLiteral` | 25 |
| if condition must conform to `BooleanConditional`, found `_` | 18 |
| member not found: `equals` on type `<error>` | 13 |
| cannot access member on type '[UInt8]' | 12 |
| no method 'write' on type 'H' matches the provided arguments | 11 |
| no matching initializer for struct 'UInt32' | 10 |
| no matching initializer for struct 'UInt8' | 10 |
| cannot find type 'lang' in this scope | 9 |
| member 'raw' is private and not accessible from this scope | 7 |
| member 'substringBytes' is private and not accessible from this scope | 6 |
| member not found: `shiftLeft` on type `<error>` | 6 |
| non-exhaustive match expression | 6 |
| type mismatch: expected `T`, found `Self` | 6 |
| member not found: `equals` on type `T` | 5 |
| no matching overload for 'array' with 1 argument(s) | 5 |
| cannot find type 'Array' in this scope | 4 |
| cannot use 'self' in free function | 4 |
| type mismatch: expected `Optional[T]`, found `Self` | 4 |
| could not infer type for 1 placeholder(s) | 3 |
| no method 'buffer' on type 'StringStorage[A]' | 3 |
| no method 'byteAt' on type 'String[A]' matches the provided arguments | 3 |
| undeclared type parameter 'T' in where clause | 3 |
| undefined name 'Dictionary' | 3 |
| cannot access member on type 'I16' | 2 |
| cannot access member on type 'I32' | 2 |
| cannot access member on type 'I64' | 2 |
| cannot access member on type 'Self' | 2 |
| cannot assign to immutable field 'length' | 2 |
| cannot assign to immutable field 'pointee' | 2 |
| cannot find type 'DictionaryIterator' in this scope | 2 |
| cannot find type 'I' in this scope | 2 |
| initializer does not initialize all fields: 'cap', 'ptr' | 2 |
| no matching overload for 'decodeUtf8' with 2 argument(s) | 2 |
| no method 'allocate' on type 'A' matches the provided arguments | 2 |
| no method 'ensureCapacity' on type 'String' matches the provided arguments | 2 |
| symbol 'Array' not found in module 'std.collections' | 2 |
| type mismatch: expected `Ordering`, found `Self` | 2 |
| 'Equal' is not a type | 1 |
| 'Optional' is not a protocol; bound must be a protocol | 1 |
| Parse error: found 'Underscore' at 2353..2354 expected something else, 'Mutating', 'Consuming', or 'RParen' | 1 |
| Parse error: found 'Underscore' at 6127..6128 expected something else, 'Mutating', 'Consuming', or 'RParen' | 1 |
| cannot access member on type 'I.Item' | 1 |
| cannot access member on type 'Inner.Iter' | 1 |
| cannot access member on type 'Int' | 1 |
| cannot call 'clone' on type 'T' | 1 |
| cannot find type 'Dictionary' in this scope | 1 |
| cannot find type 'NonCopyable' in this scope | 1 |
| initializer does not initialize all fields: 'ptr' | 1 |
| initializer does not initialize all fields: 'storage' | 1 |
| member 'storage' is private and not accessible from this scope | 1 |
| member not found: `None` on type `Self` | 1 |
| member not found: `Some` on type `Self` | 1 |
| member not found: `add` on type `String` | 1 |
| member not found: `logicalNot` on type `<error>` | 1 |
| no matching initializer for struct 'ArcBox' | 1 |
| no matching initializer for struct 'UInt64' | 1 |
| no member 'toBytes' on type 'UInt64' | 1 |
| no method 'action' on type 'InspectIterator' | 1 |
| no method 'deallocate' on type 'A' matches the provided arguments | 1 |
| no method 'insert' on type 'Set' matches the provided arguments | 1 |
| no method 'predicate' on type 'FilterIterator' | 1 |
| no method 'predicate' on type 'SkipWhileIterator' | 1 |
| no method 'predicate' on type 'TakeWhileIterator' | 1 |
| no method 'reallocate' on type 'A' matches the provided arguments | 1 |
| no method 'transform' on type 'FilterMapIterator' | 1 |
| no method 'transform' on type 'FlatMapIterator' | 1 |
| struct `SplitIterator` has Cloneable field `string` but does not conform to Cloneable | 1 |
| symbol 'Dictionary' not found in module 'std.collections' | 1 |
| symbol 'DictionaryIterator' not found in module 'std.collections' | 1 |
| symbol 'Nil' not found in module 'std.ops' | 1 |
| symbol 'NonCopyable' not found in module 'std.ops' | 1 |
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
| type mismatch: expected `Optional[I.Item]`, found `Optional[Optional[I.Item]]` | 1 |
| type mismatch: expected `Optional[_]`, found `Self` | 1 |
| type mismatch: expected `Result[_, _]`, found `Self` | 1 |
| type mismatch: expected `Self`, found `Optional[_]` | 1 |
| type mismatch: expected `Self`, found `Result[_, _]` | 1 |
| undefined name 'bitwiseAnd' | 1 |
| undefined name 'bitwiseNot' | 1 |

2. Details

undefined name 'lang'
Call Sites
- lang/std/result/result.ks:58:28
- lang/std/result/result.ks:78:27
- lang/std/result/result.ks:87:28
- lang/std/result/result.ks:93:23
- lang/std/result/optional.ks:47:22
- lang/std/result/optional.ks:69:22
- lang/std/iter/adapters.ks:98:9
- lang/std/core/bool.ks:27:27
- lang/std/core/bool.ks:45:27
- lang/std/core/bool.ks:50:27
- lang/std/core/bool.ks:55:27
- lang/std/memory/buffer.ks:23:22
- lang/std/memory/buffer.ks:35:22
- lang/std/memory/buffer.ks:111:9
- lang/std/memory/buffer.ks:111:79
- lang/std/memory/buffer.ks:117:9
- lang/std/memory/buffer.ks:117:80
- lang/std/memory/buffer.ks:121:9
- lang/std/memory/buffer.ks:121:57
- lang/std/memory/buffer.ks:134:22
- lang/std/memory/buffer.ks:169:22
- lang/std/memory/buffer.ks:182:9
- lang/std/memory/buffer.ks:191:12
- lang/std/memory/allocator.ks:36:19
- lang/std/memory/allocator.ks:37:12
- lang/std/memory/allocator.ks:45:9
- lang/std/memory/allocator.ks:49:22
- lang/std/memory/allocator.ks:50:12
- lang/std/memory/pointer.ks:20:20
- lang/std/memory/pointer.ks:24:25
- lang/std/memory/pointer.ks:28:9
- lang/std/memory/pointer.ks:32:9
- lang/std/memory/pointer.ks:36:22
- lang/std/memory/pointer.ks:40:25
- lang/std/memory/pointer.ks:57:20
- lang/std/memory/pointer.ks:61:22
- lang/std/memory/pointer.ks:61:39
- lang/std/memory/pointer.ks:65:15
- lang/std/memory/pointer.ks:66:15
- lang/std/memory/pointer.ks:70:9
- lang/std/memory/pointer.ks:70:29
- lang/std/memory/pointer.ks:74:9
- lang/std/memory/pointer.ks:74:26
- lang/std/memory/pointer.ks:78:9
- lang/std/memory/pointer.ks:82:9
- lang/std/memory/pointer.ks:86:22
- lang/std/memory/pointer.ks:86:39
- lang/std/memory/pointer.ks:86:69
- lang/std/memory/pointer.ks:90:25
- lang/std/memory/layout.ks:18:22
- lang/std/memory/layout.ks:18:51

type `Self` does not conform to protocol `ExpressibleByIntLiteral`
Call Sites
- lang/std/iter/adapters.ks:135:29
- lang/std/iter/adapters.ks:186:32
- lang/std/iter/adapters.ks:492:29
- lang/std/memory/buffer.ks:144:21
- lang/std/memory/pointer.ks:139:21
- lang/std/memory/pointer.ks:153:23
- lang/std/memory/pointer.ks:161:23
- lang/std/memory/pointer.ks:193:29
- lang/std/text/string.ks:113:34
- lang/std/text/string.ks:234:21
- lang/std/text/char.ks:24:22
- lang/std/text/char.ks:28:24
- lang/std/text/char.ks:28:45
- lang/std/text/char.ks:29:24
- lang/std/text/char.ks:29:45
- lang/std/text/char.ks:34:23
- lang/std/text/char.ks:34:44
- lang/std/text/char.ks:50:22
- lang/std/text/char.ks:54:23
- lang/std/text/char.ks:54:44
- lang/std/text/char.ks:58:23
- lang/std/text/char.ks:58:44
- lang/std/text/char.ks:79:25
- lang/std/text/char.ks:80:30
- lang/std/text/char.ks:81:30

if condition must conform to `BooleanConditional`, found `_`
Call Sites
- lang/std/iter/adapters.ks:216:16
- lang/std/memory/buffer.ks:191:12
- lang/std/memory/allocator.ks:37:12
- lang/std/memory/allocator.ks:50:12
- lang/std/ops/range.ks:127:19
- lang/std/text/views.ks:222:16
- lang/std/text/views.ks:226:23
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

member not found: `equals` on type `<error>`
Call Sites
- lang/std/text/views.ks:222:16
- lang/std/text/views.ks:226:23
- lang/std/text/views.ks:230:63
- lang/std/text/string.ks:223:16
- lang/std/text/string.ks:223:30
- lang/std/text/string.ks:223:43
- lang/std/text/string.ks:223:57
- lang/std/text/string.ks:236:16
- lang/std/text/string.ks:236:30
- lang/std/text/string.ks:236:43
- lang/std/text/string.ks:236:57
- lang/std/text/char.ks:146:9
- lang/std/text/char.ks:160:9

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

cannot find type 'lang' in this scope
Call Sites
- lang/std/core/bool.ks:18:24
- lang/std/memory/pointer.ks:13:31
- lang/std/memory/pointer.ks:15:31
- lang/std/memory/pointer.ks:70:43
- lang/std/memory/pointer.ks:70:43
- lang/std/memory/pointer.ks:74:40
- lang/std/memory/pointer.ks:74:40
- lang/std/memory/pointer.ks:90:39
- lang/std/memory/pointer.ks:90:39

member 'raw' is private and not accessible from this scope
Call Sites
- lang/std/memory/buffer.ks:111:38
- lang/std/memory/buffer.ks:111:62
- lang/std/memory/buffer.ks:117:39
- lang/std/memory/buffer.ks:117:63
- lang/std/memory/buffer.ks:121:38
- lang/std/memory/allocator.ks:45:26
- lang/std/memory/allocator.ks:49:39

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

non-exhaustive match expression
Call Sites
- lang/std/result/result.ks:171:9
- lang/std/memory/buffer.ks:18:9
- lang/std/memory/buffer.ks:30:9
- lang/std/memory/buffer.ks:129:9
- lang/std/memory/buffer.ks:164:9
- lang/std/text/char.ks:87:9

type mismatch: expected `T`, found `Self`
Call Sites
- lang/std/ops/range.ks:36:9
- lang/std/ops/range.ks:36:33
- lang/std/ops/range.ks:69:12
- lang/std/ops/range.ks:90:9
- lang/std/ops/range.ks:90:33
- lang/std/ops/range.ks:130:19

member not found: `equals` on type `T`
Call Sites
- lang/std/ops/range.ks:44:9
- lang/std/ops/range.ks:44:39
- lang/std/ops/range.ks:98:9
- lang/std/ops/range.ks:98:39
- lang/std/ops/range.ks:127:19

no matching overload for 'array' with 1 argument(s)
Call Sites
- lang/std/memory/buffer.ks:17:22
- lang/std/memory/buffer.ks:29:22
- lang/std/memory/buffer.ks:47:22
- lang/std/memory/buffer.ks:126:25
- lang/std/memory/buffer.ks:127:25

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

type mismatch: expected `Optional[T]`, found `Self`
Call Sites
- lang/std/result/optional.ks:155:9
- lang/std/result/optional.ks:155:9
- lang/std/result/optional.ks:161:9
- lang/std/result/optional.ks:161:9

could not infer type for 1 placeholder(s)
Call Sites
- lang/std/iter/adapters.ks:61:20
- lang/std/iter/adapters.ks:92:20
- lang/std/text/string.ks:381:20

no method 'buffer' on type 'StringStorage[A]'
Call Sites
- lang/std/text/string.ks:222:24
- lang/std/text/string.ks:235:24
- lang/std/text/string.ks:351:9

no method 'byteAt' on type 'String[A]' matches the provided arguments
Call Sites
- lang/std/text/views.ks:77:24
- lang/std/text/views.ks:221:24
- lang/std/text/views.ks:230:63

undeclared type parameter 'T' in where clause
Call Sites
- lang/std/result/result.ks:170:61
- lang/std/memory/buffer.ks:186:48
- lang/std/collections/set.ks:195:44

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

cannot access member on type 'Self'
Call Sites
- lang/std/result/result.ks:152:9
- lang/std/result/optional.ks:126:9

cannot assign to immutable field 'length'
Call Sites
- lang/std/text/string.ks:146:9
- lang/std/text/string.ks:345:9

cannot assign to immutable field 'pointee'
Call Sites
- lang/std/memory/buffer.ks:167:17
- lang/std/memory/allocator.ks:146:9

cannot find type 'DictionaryIterator' in this scope
Call Sites
- lang/std/collections/set.ks:237:27
- lang/std/collections/set.ks:239:27

cannot find type 'I' in this scope
Call Sites
- lang/std/iter/iterator.ks:22:24
- lang/std/collections/set.ks:39:31

initializer does not initialize all fields: 'cap', 'ptr'
Call Sites
- lang/std/memory/buffer.ks:15:5
- lang/std/memory/buffer.ks:27:5

no matching overload for 'decodeUtf8' with 2 argument(s)
Call Sites
- lang/std/text/views.ks:127:28
- lang/std/text/string.ks:381:32

no method 'allocate' on type 'A' matches the provided arguments
Call Sites
- lang/std/memory/buffer.ks:18:15
- lang/std/memory/buffer.ks:30:15

no method 'ensureCapacity' on type 'String' matches the provided arguments
Call Sites
- lang/std/text/string.ks:127:9
- lang/std/text/string.ks:137:9

symbol 'Array' not found in module 'std.collections'
Call Sites
- lang/std/text/string.ks:8:25
- lang/std/text/char.ks:7:25

type mismatch: expected `Ordering`, found `Self`
Call Sites
- lang/std/core/ordering.ks:46:18
- lang/std/core/ordering.ks:53:18

'Equal' is not a type
Call Sites
- lang/std/core/ordering.ks:8:34

'Optional' is not a protocol; bound must be a protocol
Call Sites
- lang/std/result/result.ks:170:64

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

cannot call 'clone' on type 'T'
Call Sites
- lang/std/memory/buffer.ks:187:23

cannot find type 'Dictionary' in this scope
Call Sites
- lang/std/collections/set.ks:23:23

cannot find type 'NonCopyable' in this scope
Call Sites
- lang/std/memory/buffer.ks:9:29

initializer does not initialize all fields: 'ptr'
Call Sites
- lang/std/memory/buffer.ks:161:5

initializer does not initialize all fields: 'storage'
Call Sites
- lang/std/text/string.ks:52:5

member 'storage' is private and not accessible from this scope
Call Sites
- lang/std/text/views.ks:56:36

member not found: `None` on type `Self`
Call Sites
- lang/std/result/optional.ks:154:16

member not found: `Some` on type `Self`
Call Sites
- lang/std/result/optional.ks:160:16

member not found: `add` on type `String`
Call Sites
- lang/std/result/result.ks:58:39

member not found: `logicalNot` on type `<error>`
Call Sites
- lang/std/iter/adapters.ks:216:16

no matching initializer for struct 'ArcBox'
Call Sites
- lang/std/memory/buffer.ks:183:9

no matching initializer for struct 'UInt64'
Call Sites
- lang/std/core/protocols.ks:110:37

no member 'toBytes' on type 'UInt64'
Call Sites
- lang/std/collections/set.ks:229:42

no method 'action' on type 'InspectIterator'
Call Sites
- lang/std/iter/adapters.ks:116:13

no method 'deallocate' on type 'A' matches the provided arguments
Call Sites
- lang/std/memory/buffer.ks:48:9

no method 'insert' on type 'Set' matches the provided arguments
Call Sites
- lang/std/collections/set.ks:42:13

no method 'predicate' on type 'FilterIterator'
Call Sites
- lang/std/iter/adapters.ks:39:16

no method 'predicate' on type 'SkipWhileIterator'
Call Sites
- lang/std/iter/adapters.ks:216:20

no method 'predicate' on type 'TakeWhileIterator'
Call Sites
- lang/std/iter/adapters.ks:164:16

no method 'reallocate' on type 'A' matches the provided arguments
Call Sites
- lang/std/memory/buffer.ks:129:15

no method 'transform' on type 'FilterMapIterator'
Call Sites
- lang/std/iter/adapters.ks:61:29

no method 'transform' on type 'FlatMapIterator'
Call Sites
- lang/std/iter/adapters.ks:93:38

struct `SplitIterator` has Cloneable field `string` but does not conform to Cloneable
Call Sites
- lang/std/text/string.ks:359:5

symbol 'Dictionary' not found in module 'std.collections'
Call Sites
- lang/std/collections/set.ks:9:25

symbol 'DictionaryIterator' not found in module 'std.collections'
Call Sites
- lang/std/collections/set.ks:9:37

symbol 'Nil' not found in module 'std.ops'
Call Sites
- lang/std/result/optional.ks:7:42

symbol 'NonCopyable' not found in module 'std.ops'
Call Sites
- lang/std/memory/buffer.ks:7:17

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

type mismatch: expected `Optional[I.Item]`, found `Optional[Optional[I.Item]]`
Call Sites
- lang/std/iter/adapters.ks:394:20

type mismatch: expected `Optional[_]`, found `Self`
Call Sites
- lang/std/result/optional.ks:166:33

type mismatch: expected `Result[_, _]`, found `Self`
Call Sites
- lang/std/result/result.ks:180:31

type mismatch: expected `Self`, found `Optional[_]`
Call Sites
- lang/std/result/optional.ks:166:26

type mismatch: expected `Self`, found `Result[_, _]`
Call Sites
- lang/std/result/result.ks:180:24

undefined name 'bitwiseAnd'
Call Sites
- lang/std/memory/allocator.ks:73:66

undefined name 'bitwiseNot'
Call Sites
- lang/std/memory/allocator.ks:73:100
