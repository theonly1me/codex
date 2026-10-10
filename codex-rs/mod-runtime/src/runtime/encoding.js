const alphabet =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export function btoa(value) {
  if (typeof value !== "string" || value.length > 4 * 1024 * 1024)
    throw Error("invalid base64 input");
  let encoded = "";
  for (let index = 0; index < value.length; index += 3) {
    const first = value.charCodeAt(index);
    const second = value.charCodeAt(index + 1);
    const third = value.charCodeAt(index + 2);
    if (first > 255 || second > 255 || third > 255)
      throw Error("base64 input must contain bytes");
    const bits = (first << 16) | ((second || 0) << 8) | (third || 0);
    encoded += alphabet[(bits >>> 18) & 63] + alphabet[(bits >>> 12) & 63];
    encoded += index + 1 < value.length ? alphabet[(bits >>> 6) & 63] : "=";
    encoded += index + 2 < value.length ? alphabet[bits & 63] : "=";
  }
  return encoded;
}

export function atob(value) {
  if (
    typeof value !== "string" ||
    value.length % 4 !== 0 ||
    !/^[A-Za-z0-9+/]*={0,2}$/.test(value)
  )
    throw Error("invalid base64 input");
  let decoded = "";
  for (let index = 0; index < value.length; index += 4) {
    const bits =
      (alphabet.indexOf(value[index]) << 18) |
      (alphabet.indexOf(value[index + 1]) << 12) |
      (Math.max(0, alphabet.indexOf(value[index + 2])) << 6) |
      Math.max(0, alphabet.indexOf(value[index + 3]));
    decoded += String.fromCharCode((bits >>> 16) & 255);
    if (value[index + 2] !== "=")
      decoded += String.fromCharCode((bits >>> 8) & 255);
    if (value[index + 3] !== "=") decoded += String.fromCharCode(bits & 255);
  }
  return decoded;
}
