package gov.fnal.controls.common.epics.v1;

import static org.junit.jupiter.api.Assertions.*;

import com.google.protobuf.Descriptors;
import java.util.HexFormat;
import org.junit.jupiter.api.Test;

/** Exercises generated APIs and Python-produced bytes, not live pvData conversion. */
class WireContractTest {
    @Test
    void parsesPythonUnsignedScalarAndPresentDefaults() throws Exception {
        // Produced with Python protobuf 6.30.2 and protoc 30.2.
        byte[] pythonBytes = HexFormat.of().parseHex(
                "0a0b48ffffffffffffffffff011200221808ffffffffffffffffff01"
                + "10fbffffffffffffffff01182aa2061565706963733a6e742f4e545363616c61723a312e30");
        NTScalar message = NTScalar.parseFrom(pythonBytes);
        assertEquals("epics:nt/NTScalar:1.0", message.getTypeId());
        assertEquals(Scalar.ValueCase.ULONG_VALUE, message.getValue().getValueCase());
        assertEquals("18446744073709551615", Long.toUnsignedString(message.getValue().getUlongValue()));
        assertTrue(message.hasDescriptorText());
        assertEquals("", message.getDescriptorText());
        assertTrue(message.getTimeStamp().hasUserTag());
        assertEquals(42, message.getTimeStamp().getUserTag());
        assertEquals(-1, message.getTimeStamp().getSecondsPastEpoch());
        assertEquals(-5, message.getTimeStamp().getNanoseconds());
        assertArrayEquals(pythonBytes, message.toByteArray());
    }

    @Test
    void retainsSelectedEmptyArrayAndOptionalPresence() throws Exception {
        NTScalarArray message = NTScalarArray.newBuilder()
                .setTypeId("epics:nt/NTScalarArray:1.0")
                .setValue(ScalarArray.newBuilder().setDoubleValue(DoubleArray.getDefaultInstance()))
                .setDescriptorText("")
                .build();
        NTScalarArray copy = NTScalarArray.parseFrom(message.toByteArray());
        assertTrue(copy.hasValue());
        assertTrue(copy.hasDescriptorText());
        assertEquals(ScalarArray.ValueCase.DOUBLE_VALUE, copy.getValue().getValueCase());
        assertEquals(0, copy.getValue().getDoubleValue().getValueCount());
        assertFalse(copy.hasTimeStamp());
    }

    @Test
    void exposesAllThirteenTypesAndStableTransportMetadataTags() {
        Descriptors.FileDescriptor file = EpicsNormativeTypesProto.getDescriptor();
        long count = file.getMessageTypes().stream()
                .filter(type -> type.getName().startsWith("NT"))
                .peek(type -> {
                    assertEquals(100, type.findFieldByName("type_id").getNumber());
                    assertTrue(type.findFieldByName("type_id").hasPresence());
                    assertEquals(101, type.findFieldByName("extra_fields").getNumber());
                }).count();
        assertEquals(13, count);
        // descriptor_text is intentional; descriptor would collide with Java's
        // static protobuf getDescriptor() method.
        assertEquals(2, NTScalar.getDescriptor().findFieldByName("descriptor_text").getNumber());
    }
}
