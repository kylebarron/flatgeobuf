package org.wololo.flatgeobuf.generated;
// automatically generated by the FlatBuffers compiler, do not modify

import java.nio.*;
import java.lang.*;
import java.util.*;
import com.google.flatbuffers.*;

@SuppressWarnings("unused")
public final class Column extends Table {
  public static void ValidateVersion() { Constants.FLATBUFFERS_1_12_0(); }
  public static Column getRootAsColumn(ByteBuffer _bb) { return getRootAsColumn(_bb, new Column()); }
  public static Column getRootAsColumn(ByteBuffer _bb, Column obj) { _bb.order(ByteOrder.LITTLE_ENDIAN); return (obj.__assign(_bb.getInt(_bb.position()) + _bb.position(), _bb)); }
  public void __init(int _i, ByteBuffer _bb) { __reset(_i, _bb); }
  public Column __assign(int _i, ByteBuffer _bb) { __init(_i, _bb); return this; }

  public String name() { int o = __offset(4); return o != 0 ? __string(o + bb_pos) : null; }
  public ByteBuffer nameAsByteBuffer() { return __vector_as_bytebuffer(4, 1); }
  public ByteBuffer nameInByteBuffer(ByteBuffer _bb) { return __vector_in_bytebuffer(_bb, 4, 1); }
  public int type() { int o = __offset(6); return o != 0 ? bb.get(o + bb_pos) & 0xFF : 0; }

  public static int createColumn(FlatBufferBuilder builder,
      int nameOffset,
      int type) {
    builder.startTable(2);
    Column.addName(builder, nameOffset);
    Column.addType(builder, type);
    return Column.endColumn(builder);
  }

  public static void startColumn(FlatBufferBuilder builder) { builder.startTable(2); }
  public static void addName(FlatBufferBuilder builder, int nameOffset) { builder.addOffset(0, nameOffset, 0); }
  public static void addType(FlatBufferBuilder builder, int type) { builder.addByte(1, (byte)type, (byte)0); }
  public static int endColumn(FlatBufferBuilder builder) {
    int o = builder.endTable();
    builder.required(o, 4);  // name
    return o;
  }

  public static final class Vector extends BaseVector {
    public Vector __assign(int _vector, int _element_size, ByteBuffer _bb) { __reset(_vector, _element_size, _bb); return this; }

    public Column get(int j) { return get(new Column(), j); }
    public Column get(Column obj, int j) {  return obj.__assign(__indirect(__element(j), bb), bb); }
  }
}

