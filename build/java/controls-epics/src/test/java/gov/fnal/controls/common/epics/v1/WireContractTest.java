package gov.fnal.controls.common.epics.v1;

import static org.junit.jupiter.api.Assertions.*;

import com.google.protobuf.Descriptors;
import com.google.protobuf.UnknownFieldSet;
import java.util.HexFormat;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

/** Exercises generated APIs and Rust-produced bytes, not live pvData conversion. */
class WireContractTest {
    @Test
    void parsesRustUnsignedScalarAndPresentDefaults() throws Exception {
        // CI supplies bytes written by the generated Rust/Prost binding. The
        // fixed reference also lets legacy Java users run Maven independently.
        byte[] reference = HexFormat.of().parseHex(
                "0a0b48ffffffffffffffffff011200221808ffffffffffffffffff01"
                + "10fbffffffffffffffff01182aa2061565706963733a6e742f4e545363616c61723a312e30");
        String fixture = System.getProperty("epics.nt.fixture");
        byte[] wireBytes = fixture == null ? reference : Files.readAllBytes(Path.of(fixture));
        assertArrayEquals(reference, wireBytes);
        NTScalar message = NTScalar.parseFrom(wireBytes);
        assertEquals("epics:nt/NTScalar:1.0", message.getTypeId());
        assertEquals(Scalar.ValueCase.ULONG_VALUE, message.getValue().getValueCase());
        assertEquals("18446744073709551615", Long.toUnsignedString(message.getValue().getUlongValue()));
        assertTrue(message.hasDescriptorText());
        assertEquals("", message.getDescriptorText());
        assertTrue(message.getTimeStamp().hasUserTag());
        assertEquals(42, message.getTimeStamp().getUserTag());
        assertEquals(-1, message.getTimeStamp().getSecondsPastEpoch());
        assertEquals(-5, message.getTimeStamp().getNanoseconds());
        assertArrayEquals(wireBytes, message.toByteArray());
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
    void fullJavaRuntimeRetainsUnknownFields() throws Exception {
        UnknownFieldSet unknown = UnknownFieldSet.newBuilder()
                .addField(200, UnknownFieldSet.Field.newBuilder().addVarint(7).build())
                .build();
        NTScalar message = NTScalar.newBuilder()
                .setTypeId("epics:nt/NTScalar:1.0")
                .setValue(Scalar.newBuilder().setIntValue(0))
                .setUnknownFields(unknown)
                .build();
        NTScalar restored = NTScalar.parseFrom(message.toByteArray());
        assertEquals(unknown, restored.getUnknownFields());
        assertArrayEquals(message.toByteArray(), restored.toByteArray());
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
