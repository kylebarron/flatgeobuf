// <auto-generated>
//  automatically generated by the FlatBuffers compiler, do not modify
// </auto-generated>

namespace FlatGeobuf
{

using global::System;
using global::FlatBuffers;

public struct Column : IFlatbufferObject
{
  private Table __p;
  public ByteBuffer ByteBuffer { get { return __p.bb; } }
  public static Column GetRootAsColumn(ByteBuffer _bb) { return GetRootAsColumn(_bb, new Column()); }
  public static Column GetRootAsColumn(ByteBuffer _bb, Column obj) { return (obj.__assign(_bb.GetInt(_bb.Position) + _bb.Position, _bb)); }
  public void __init(int _i, ByteBuffer _bb) { __p.bb_pos = _i; __p.bb = _bb; }
  public Column __assign(int _i, ByteBuffer _bb) { __init(_i, _bb); return this; }

  public string Name { get { int o = __p.__offset(4); return o != 0 ? __p.__string(o + __p.bb_pos) : null; } }
#if ENABLE_SPAN_T
  public Span<byte> GetNameBytes() { return __p.__vector_as_span(4); }
#else
  public ArraySegment<byte>? GetNameBytes() { return __p.__vector_as_arraysegment(4); }
#endif
  public byte[] GetNameArray() { return __p.__vector_as_array<byte>(4); }
  public ColumnType Type { get { int o = __p.__offset(6); return o != 0 ? (ColumnType)__p.bb.Get(o + __p.bb_pos) : ColumnType.Byte; } }

  public static Offset<Column> CreateColumn(FlatBufferBuilder builder,
      StringOffset nameOffset = default(StringOffset),
      ColumnType type = ColumnType.Byte) {
    builder.StartObject(2);
    Column.AddName(builder, nameOffset);
    Column.AddType(builder, type);
    return Column.EndColumn(builder);
  }

  public static void StartColumn(FlatBufferBuilder builder) { builder.StartObject(2); }
  public static void AddName(FlatBufferBuilder builder, StringOffset nameOffset) { builder.AddOffset(0, nameOffset.Value, 0); }
  public static void AddType(FlatBufferBuilder builder, ColumnType type) { builder.AddByte(1, (byte)type, 0); }
  public static Offset<Column> EndColumn(FlatBufferBuilder builder) {
    int o = builder.EndObject();
    builder.Required(o, 4);  // name
    return new Offset<Column>(o);
  }
};


}
