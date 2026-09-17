package org.meshchat.transport

import org.junit.Assert.*
import org.junit.Test

class BeaconPolicyTest {
    @Test fun continuousLowLatencyStillRespectsPermissionAndFailureBackoff() {
        var now=0uL
        val policy=ScanPolicy {now}
        assertEquals(ScanMode.BURST,policy.mode(false,8,false,true,true))
        now=6uL*60uL*60uL*1000uL
        assertEquals(ScanMode.BURST,policy.mode(false,0,false,true,true))
        assertEquals(ScanMode.OFF,policy.mode(false,8,false,false,true))
        policy.failed()
        assertEquals(ScanMode.OFF,policy.mode(false,8,false,true,true))
        now+=5000uL
        assertEquals(ScanMode.BURST,policy.mode(false,8,false,true,true))
        assertEquals(ScanMode.BALANCED,policy.mode(false,8,false,true,false))
    }
    @Test fun sparsePeerClaimIsOnlyBoundedPreferenceAndDoesNotEvadeIdleOrSlotLimits() {
        var now=0uL
        val policy=ConnectionPolicy {now}
        fun address(i:Int)="00:00:00:00:00:%02X".format(i)
        val peers=(1..8).map { i ->
            policy.discovered(address(i),null)
            ConnectionInfo(i.toLong(),address(i),0uL,1uL,if(i==1)0 else 900,if(i==1)0 else 8)
        }
        now=60_000uL
        policy.discovered(address(9),null)
        val plan=policy.plan(peers,8,true)
        assertFalse(plan.close.contains(1L))
        assertTrue(plan.close.size<=2)
        assertFalse(policy.allowed(address(10),peers,8,true))
        val idle=peers.map {it.copy(validAt=null,peerCount=0)}
        assertEquals(8,policy.plan(idle,8,true).close.size)
    }
}
