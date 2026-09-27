package org.meshchat.ui

import org.junit.Assert.*
import org.junit.Test

class MeasurementPlanTest {
    @Test fun acceptanceHasThirtyEachInBothDirectionsWithoutBackground() {
        for(role in listOf("A", "B")) {
            val plan=MeasurementPlan("acceptance",role)
            assertEquals(60,plan.pairs)
            assertEquals(30,(0 until plan.pairs).count {plan.long(it)})
            assertFalse((0 until plan.pairs).any {plan.background(it)})
        }
    }
    @Test fun smokeRetainsHistoricalScheduleAndRejectsInvalidPlans() {
        val plan=MeasurementPlan("smoke","A")
        assertEquals(listOf(3,4,5),(0 until plan.pairs).filter {plan.long(it)})
        assertEquals(listOf(6),(0 until plan.pairs).filter {plan.background(it)})
        assertThrows(IllegalArgumentException::class.java) {MeasurementPlan("acceptance","C")}
        assertThrows(IllegalArgumentException::class.java) {MeasurementPlan("smoke","B")}
        assertThrows(IllegalArgumentException::class.java) {MeasurementPlan("fast","A")}
        assertThrows(IllegalArgumentException::class.java) {plan.long(7)}
    }
}
