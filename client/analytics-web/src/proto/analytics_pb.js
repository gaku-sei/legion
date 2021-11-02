// source: analytics.proto
/**
 * @fileoverview
 * @enhanceable
 * @suppress {missingRequire} reports error on implicit type usages.
 * @suppress {messageConventions} JS Compiler reports an error if a variable or
 *     field starts with 'MSG_' and isn't a translatable message.
 * @public
 */
// GENERATED CODE -- DO NOT EDIT!
/* eslint-disable */
// @ts-nocheck

var jspb = require('google-protobuf');
var goog = jspb;
var global = (function() { return this || window || global || self || Function('return this')(); }).call(null);

var process_pb = require('./process_pb.js');
goog.object.extend(proto, process_pb);
var stream_pb = require('./stream_pb.js');
goog.object.extend(proto, stream_pb);
var block_pb = require('./block_pb.js');
goog.object.extend(proto, block_pb);
goog.exportSymbol('proto.analytics.FindProcessReply', null, global);
goog.exportSymbol('proto.analytics.FindProcessRequest', null, global);
goog.exportSymbol('proto.analytics.ListProcessStreamsRequest', null, global);
goog.exportSymbol('proto.analytics.ListStreamsReply', null, global);
goog.exportSymbol('proto.analytics.ProcessListReply', null, global);
goog.exportSymbol('proto.analytics.RecentProcessesRequest', null, global);
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.FindProcessRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.analytics.FindProcessRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.FindProcessRequest.displayName = 'proto.analytics.FindProcessRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.FindProcessReply = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.analytics.FindProcessReply, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.FindProcessReply.displayName = 'proto.analytics.FindProcessReply';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.RecentProcessesRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.analytics.RecentProcessesRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.RecentProcessesRequest.displayName = 'proto.analytics.RecentProcessesRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.ProcessListReply = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.analytics.ProcessListReply.repeatedFields_, null);
};
goog.inherits(proto.analytics.ProcessListReply, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.ProcessListReply.displayName = 'proto.analytics.ProcessListReply';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.ListProcessStreamsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.analytics.ListProcessStreamsRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.ListProcessStreamsRequest.displayName = 'proto.analytics.ListProcessStreamsRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.analytics.ListStreamsReply = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.analytics.ListStreamsReply.repeatedFields_, null);
};
goog.inherits(proto.analytics.ListStreamsReply, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.analytics.ListStreamsReply.displayName = 'proto.analytics.ListStreamsReply';
}



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.FindProcessRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.FindProcessRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.FindProcessRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.FindProcessRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    processId: jspb.Message.getFieldWithDefault(msg, 1, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.FindProcessRequest}
 */
proto.analytics.FindProcessRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.FindProcessRequest;
  return proto.analytics.FindProcessRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.FindProcessRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.FindProcessRequest}
 */
proto.analytics.FindProcessRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setProcessId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.FindProcessRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.FindProcessRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.FindProcessRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.FindProcessRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProcessId();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
};


/**
 * optional string process_id = 1;
 * @return {string}
 */
proto.analytics.FindProcessRequest.prototype.getProcessId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.analytics.FindProcessRequest} returns this
 */
proto.analytics.FindProcessRequest.prototype.setProcessId = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.FindProcessReply.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.FindProcessReply.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.FindProcessReply} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.FindProcessReply.toObject = function(includeInstance, msg) {
  var f, obj = {
    process: (f = msg.getProcess()) && process_pb.Process.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.FindProcessReply}
 */
proto.analytics.FindProcessReply.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.FindProcessReply;
  return proto.analytics.FindProcessReply.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.FindProcessReply} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.FindProcessReply}
 */
proto.analytics.FindProcessReply.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new process_pb.Process;
      reader.readMessage(value,process_pb.Process.deserializeBinaryFromReader);
      msg.setProcess(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.FindProcessReply.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.FindProcessReply.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.FindProcessReply} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.FindProcessReply.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProcess();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      process_pb.Process.serializeBinaryToWriter
    );
  }
};


/**
 * optional telemetry.Process process = 1;
 * @return {?proto.telemetry.Process}
 */
proto.analytics.FindProcessReply.prototype.getProcess = function() {
  return /** @type{?proto.telemetry.Process} */ (
    jspb.Message.getWrapperField(this, process_pb.Process, 1));
};


/**
 * @param {?proto.telemetry.Process|undefined} value
 * @return {!proto.analytics.FindProcessReply} returns this
*/
proto.analytics.FindProcessReply.prototype.setProcess = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.analytics.FindProcessReply} returns this
 */
proto.analytics.FindProcessReply.prototype.clearProcess = function() {
  return this.setProcess(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.analytics.FindProcessReply.prototype.hasProcess = function() {
  return jspb.Message.getField(this, 1) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.RecentProcessesRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.RecentProcessesRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.RecentProcessesRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.RecentProcessesRequest.toObject = function(includeInstance, msg) {
  var f, obj = {

  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.RecentProcessesRequest}
 */
proto.analytics.RecentProcessesRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.RecentProcessesRequest;
  return proto.analytics.RecentProcessesRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.RecentProcessesRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.RecentProcessesRequest}
 */
proto.analytics.RecentProcessesRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.RecentProcessesRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.RecentProcessesRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.RecentProcessesRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.RecentProcessesRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.analytics.ProcessListReply.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.ProcessListReply.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.ProcessListReply.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.ProcessListReply} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ProcessListReply.toObject = function(includeInstance, msg) {
  var f, obj = {
    processesList: jspb.Message.toObjectList(msg.getProcessesList(),
    process_pb.Process.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.ProcessListReply}
 */
proto.analytics.ProcessListReply.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.ProcessListReply;
  return proto.analytics.ProcessListReply.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.ProcessListReply} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.ProcessListReply}
 */
proto.analytics.ProcessListReply.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new process_pb.Process;
      reader.readMessage(value,process_pb.Process.deserializeBinaryFromReader);
      msg.addProcesses(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.ProcessListReply.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.ProcessListReply.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.ProcessListReply} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ProcessListReply.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProcessesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      process_pb.Process.serializeBinaryToWriter
    );
  }
};


/**
 * repeated telemetry.Process processes = 1;
 * @return {!Array<!proto.telemetry.Process>}
 */
proto.analytics.ProcessListReply.prototype.getProcessesList = function() {
  return /** @type{!Array<!proto.telemetry.Process>} */ (
    jspb.Message.getRepeatedWrapperField(this, process_pb.Process, 1));
};


/**
 * @param {!Array<!proto.telemetry.Process>} value
 * @return {!proto.analytics.ProcessListReply} returns this
*/
proto.analytics.ProcessListReply.prototype.setProcessesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.telemetry.Process=} opt_value
 * @param {number=} opt_index
 * @return {!proto.telemetry.Process}
 */
proto.analytics.ProcessListReply.prototype.addProcesses = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.telemetry.Process, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.analytics.ProcessListReply} returns this
 */
proto.analytics.ProcessListReply.prototype.clearProcessesList = function() {
  return this.setProcessesList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.ListProcessStreamsRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.ListProcessStreamsRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.ListProcessStreamsRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ListProcessStreamsRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    processId: jspb.Message.getFieldWithDefault(msg, 1, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.ListProcessStreamsRequest}
 */
proto.analytics.ListProcessStreamsRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.ListProcessStreamsRequest;
  return proto.analytics.ListProcessStreamsRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.ListProcessStreamsRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.ListProcessStreamsRequest}
 */
proto.analytics.ListProcessStreamsRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setProcessId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.ListProcessStreamsRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.ListProcessStreamsRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.ListProcessStreamsRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ListProcessStreamsRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProcessId();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
};


/**
 * optional string process_id = 1;
 * @return {string}
 */
proto.analytics.ListProcessStreamsRequest.prototype.getProcessId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.analytics.ListProcessStreamsRequest} returns this
 */
proto.analytics.ListProcessStreamsRequest.prototype.setProcessId = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.analytics.ListStreamsReply.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.analytics.ListStreamsReply.prototype.toObject = function(opt_includeInstance) {
  return proto.analytics.ListStreamsReply.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.analytics.ListStreamsReply} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ListStreamsReply.toObject = function(includeInstance, msg) {
  var f, obj = {
    streamsList: jspb.Message.toObjectList(msg.getStreamsList(),
    stream_pb.Stream.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.analytics.ListStreamsReply}
 */
proto.analytics.ListStreamsReply.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.analytics.ListStreamsReply;
  return proto.analytics.ListStreamsReply.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.analytics.ListStreamsReply} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.analytics.ListStreamsReply}
 */
proto.analytics.ListStreamsReply.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new stream_pb.Stream;
      reader.readMessage(value,stream_pb.Stream.deserializeBinaryFromReader);
      msg.addStreams(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.analytics.ListStreamsReply.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.analytics.ListStreamsReply.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.analytics.ListStreamsReply} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.analytics.ListStreamsReply.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStreamsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      stream_pb.Stream.serializeBinaryToWriter
    );
  }
};


/**
 * repeated telemetry.Stream streams = 1;
 * @return {!Array<!proto.telemetry.Stream>}
 */
proto.analytics.ListStreamsReply.prototype.getStreamsList = function() {
  return /** @type{!Array<!proto.telemetry.Stream>} */ (
    jspb.Message.getRepeatedWrapperField(this, stream_pb.Stream, 1));
};


/**
 * @param {!Array<!proto.telemetry.Stream>} value
 * @return {!proto.analytics.ListStreamsReply} returns this
*/
proto.analytics.ListStreamsReply.prototype.setStreamsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.telemetry.Stream=} opt_value
 * @param {number=} opt_index
 * @return {!proto.telemetry.Stream}
 */
proto.analytics.ListStreamsReply.prototype.addStreams = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.telemetry.Stream, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.analytics.ListStreamsReply} returns this
 */
proto.analytics.ListStreamsReply.prototype.clearStreamsList = function() {
  return this.setStreamsList([]);
};


goog.object.extend(exports, proto.analytics);
